--[[
	3D rendering: the scene shows exactly what is currently selected — staging
	Ours renders ours' version, Theirs renders theirs', unpicked renders base.
	Property values apply to the target, moves reparent it, delete-vs-edit
	swaps the stamped clone itself in and out of the scene. Unpicking restores
	boot snapshots; none of this touches the disk file — the CLI at Complete
	is still the only writer.
]]

local state = require("@self/state")

local getBoundingBox = require("@self/viewport/getBoundingBox")
local getIdealCameraCFrame = require("@self/viewport/getIdealCameraCFrame")

type Conflict = state.Conflict
type SideInfo = state.SideInfo

local preview = {}

local function eachPart(root: Instance, fn: (BasePart) -> ())
	if root:IsA("BasePart") then
		fn(root)
	end
	for _, descendant in root:GetDescendants() do
		if descendant:IsA("BasePart") then
			fn(descendant)
		end
	end
end

local function sideInfo(entry: Instance, sideName: string): SideInfo
	local info: SideInfo = {}
	local folder = entry:FindFirstChild(sideName)
	if folder then
		info.folder = folder
		for _, child in folder:GetChildren() do
			if child.Name == "Destination" and child:IsA("ObjectValue") then
				info.destination = child.Value
			else
				info.clone = child
			end
		end
		info.destinationPath = folder:GetAttribute("DestinationPath") :: string?
	end
	return info
end

-- Boot pass: stash the container out of the scene, then per conflict collect
-- side clones + the base snapshots that unpicking restores.
function preview.boot(container: Instance)
	-- The stamped container rides inside model files, so a model preview
	-- imports it into workspace — where every clone renders on top of its
	-- target. It is pure data for this session: stash it out of the scene
	-- (copy-only; the file keeps its container until finalize strips it).
	if container:IsDescendantOf(workspace) then
		container.Parent = game:GetService("ServerStorage")
	end

	for _, conflict in state.conflicts do
		conflict.ours = sideInfo(conflict.entry, "Ours")
		conflict.theirs = sideInfo(conflict.entry, "Theirs")
		conflict.rendered = "__init"

		local target = conflict.target
		if conflict.kind == "Property" then
			conflict.baseValue = state.liveValue(conflict)
		elseif conflict.kind == "MoveTarget" then
			conflict.baseParent = target.Parent
		elseif conflict.kind == "DeleteVsEdit" then
			local parts: { [BasePart]: number } = {}
			eachPart(target, function(part)
				parts[part] = part.Transparency
			end)
			conflict.targetParts = parts
		end
	end
end

-- Frame the whole target in view — fitted distance from its bounding box and
-- the live camera FOV/aspect (vendored miu-viewport math), not a fixed offset
-- that lands inside or behind geometry depending on target size. Focus is the
-- edit camera's orbit pivot: it must be written together with the CFrame, or
-- Studio keeps steering around the stale pivot and the view drifts off target.
function preview.focusOn(conflict: Conflict)
	local camera = workspace.CurrentCamera
	local cframe, size = getBoundingBox(conflict.target :: PVInstance)
	camera.CFrame = getIdealCameraCFrame(cframe, size, {
		fov = camera.FieldOfView,
		aspectRatio = camera.ViewportSize.X / camera.ViewportSize.Y,
	})
	camera.Focus = cframe
end

-- Render the staged selection; nil (unpicked) restores the base snapshot.
local function applyPreview(conflict: Conflict)
	local staged = conflict.resolved

	if conflict.kind == "Property" then
		if staged ~= "custom" then
			local sideClone = if staged == "ours"
				then (conflict.ours :: SideInfo).clone
				elseif staged == "theirs" then (conflict.theirs :: SideInfo).clone
				else nil
			local property = conflict.property :: string
			if sideClone then
				(conflict.target :: any)[property] = (sideClone :: any)[property]
			elseif staged == nil then
				(conflict.target :: any)[property] = conflict.baseValue
			end
		end
	elseif conflict.kind == "MoveTarget" then
		local destination = if staged == "ours"
			then (conflict.ours :: SideInfo).destination
			elseif staged == "theirs" then (conflict.theirs :: SideInfo).destination
			else nil
		-- A side with no live destination (moved into the other branch's added
		-- content) previews as base rather than sticking at a stale parent.
		destination = destination or conflict.baseParent
		if destination then
			conflict.target.Parent = destination
		end
	elseif conflict.kind == "DeleteVsEdit" then
		-- One side deletes, the other edits. Staging either side hides the
		-- base target; staging the edit side additionally swaps its stamped
		-- clone into the target's spot (it carries its own true coordinates).
		local editSide = if (conflict.ours :: SideInfo).clone then "ours" else "theirs"
		local editInfo = (if editSide == "ours" then conflict.ours else conflict.theirs) :: SideInfo
		for part, original in conflict.targetParts :: { [BasePart]: number } do
			part.Transparency = if staged == nil then original else 1
		end
		local clone = editInfo.clone
		if clone then
			clone.Parent = if staged == editSide then conflict.target.Parent else editInfo.folder
		end
	end
end

function preview.renderPreviews()
	if state.finished then
		return
	end
	for _, conflict in state.conflicts do
		local staged = conflict.resolved or "unresolved"
		if conflict.rendered ~= staged then
			conflict.rendered = staged
			applyPreview(conflict)
		end
	end
end

local liveHighlights: { [string]: Highlight } = {}

function preview.updateHighlights()
	for _, conflict in state.conflicts do
		local target = conflict.target
		local spatial = target:IsDescendantOf(workspace) and target:IsA("PVInstance")
		if not spatial or state.finished then
			local existing = liveHighlights[conflict.name]
			if existing then
				existing:Destroy()
				liveHighlights[conflict.name] = nil
			end
			continue
		end

		local highlight = liveHighlights[conflict.name]
		if not highlight then
			highlight = Instance.new("Highlight")
			highlight.Name = `RbxDiffHighlight_{conflict.name}`
			highlight.Parent = target
			liveHighlights[conflict.name] = highlight
		end

		-- Adorn whatever the preview is actually rendering: normally the
		-- target, but a staged edit side of a delete-vs-edit renders its
		-- swapped-in clone instead.
		local adornee: Instance = target
		if conflict.kind == "DeleteVsEdit" then
			local editSide = if (conflict.ours :: SideInfo).clone then "ours" else "theirs"
			local info = (if editSide == "ours" then conflict.ours else conflict.theirs) :: SideInfo
			if conflict.resolved == editSide and info.clone then
				adornee = info.clone :: Instance
			end
		end
		highlight.Adornee = adornee

		local color = state.statusColor(conflict)
		highlight.OutlineColor = color
		highlight.FillColor = color
		highlight.FillTransparency = 1 -- outline only
	end
end

return preview
