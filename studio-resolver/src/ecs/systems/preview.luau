--[[
	3D rendering: the scene shows exactly what is currently selected — staging
	Ours renders ours' version, Theirs renders theirs', unpicked renders base.
	Property values apply to the target, moves reparent it, delete-vs-edit
	swaps the stamped clone itself in and out of the scene. Unpicking restores
	boot snapshots; none of this touches the disk file — the CLI at Complete
	is still the only writer.

	Everything reads/writes the world: staged Resolution vs Rendered drives
	the frame diff, side/base data live as components.
]]

local packages = require("../../packages")

local theme = require("../../theme")

local conflicts = require("./conflicts")
local cts = require("../cts")
local world = require("../world")

local getBoundingBox = require("../../viewport/getBoundingBox")
local getIdealCameraCFrame = require("../../viewport/getIdealCameraCFrame")

local jecs = packages.jecs

type SideInfo = cts.SideInfo
type Base = cts.Base

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

-- Boot pass: stash the container out of the scene, then per conflict entity
-- set side clones + the base snapshot that unpicking restores.
function preview.boot(container: Instance)
	-- The stamped container rides inside model files, so a model preview
	-- imports it into workspace — where every clone renders on top of its
	-- target. It is pure data for this session: stash it out of the scene
	-- (copy-only; the file keeps its container until finalize strips it).
	if container:IsDescendantOf(workspace) then
		container.Parent = game:GetService("ServerStorage")
	end

	for _, entity in conflicts.all() do
		local entry = world:get(entity, cts.Entry) :: Instance
		world:set(entity, cts.Ours, sideInfo(entry, "Ours"))
		world:set(entity, cts.Theirs, sideInfo(entry, "Theirs"))
		world:set(entity, cts.Rendered, "__init")

		local target = world:get(entity, cts.Target) :: Instance
		local kind = world:get(entity, cts.Kind)
		local base: Base = {}
		if kind == "Property" then
			base.value = conflicts.liveValue(entity)
		elseif kind == "MoveTarget" then
			base.parent = target.Parent
		elseif kind == "DeleteVsEdit" then
			local parts: { [BasePart]: number } = {}
			eachPart(target, function(part)
				parts[part] = part.Transparency
			end)
			base.parts = parts
		end
		world:set(entity, cts.Base, base)
	end
end

-- Frame the whole target in view — fitted distance from its bounding box and
-- the live camera FOV/aspect (vendored miu-viewport math), not a fixed offset
-- that lands inside or behind geometry depending on target size. Focus is the
-- edit camera's orbit pivot: it must be written together with the CFrame, or
-- Studio keeps steering around the stale pivot and the view drifts off target.
function preview.focusOn(entity: any)
	local camera = workspace.CurrentCamera
	local target = world:get(entity, cts.Target) :: Instance
	local cframe, size = getBoundingBox(target :: PVInstance)
	camera.CFrame = getIdealCameraCFrame(cframe, size, {
		fov = camera.FieldOfView,
		aspectRatio = camera.ViewportSize.X / camera.ViewportSize.Y,
	})
	camera.Focus = cframe
end

-- Render the staged selection; nil (unpicked) restores the base snapshot.
local function applyPreview(entity: any)
	local staged = world:get(entity, cts.Resolution) :: string?
	local kind = world:get(entity, cts.Kind)
	local target = world:get(entity, cts.Target) :: Instance
	local ours = world:get(entity, cts.Ours) :: SideInfo
	local theirs = world:get(entity, cts.Theirs) :: SideInfo
	local base = world:get(entity, cts.Base) :: Base

	if kind == "Property" then
		if staged ~= "custom" then
			local sideClone = if staged == "ours" then ours.clone elseif staged == "theirs" then theirs.clone else nil
			local property = world:get(entity, cts.Property) :: string
			if sideClone then
				(target :: any)[property] = (sideClone :: any)[property]
			elseif staged == nil then
				(target :: any)[property] = base.value
			end
		end
	elseif kind == "MoveTarget" then
		local destination = if staged == "ours"
			then ours.destination
			elseif staged == "theirs" then theirs.destination
			else nil
		-- A side with no live destination (moved into the other branch's added
		-- content) previews as base rather than sticking at a stale parent.
		destination = destination or base.parent
		if destination then
			target.Parent = destination
		end
	elseif kind == "DeleteVsEdit" then
		-- One side deletes, the other edits. Staging either side hides the
		-- base target; staging the edit side additionally swaps its stamped
		-- clone into the target's spot (it carries its own true coordinates).
		local editSide = if ours.clone then "ours" else "theirs"
		local editInfo = if editSide == "ours" then ours else theirs
		for part, original in base.parts :: { [BasePart]: number } do
			part.Transparency = if staged == nil then original else 1
		end
		local clone = editInfo.clone
		if clone then
			clone.Parent = if staged == editSide then target.Parent else editInfo.folder
		end
	end
end

function preview.renderPreviews()
	if world:has(cts.Finished, cts.Finished) then
		return
	end
	for _, entity in conflicts.all() do
		local staged = (world:get(entity, cts.Resolution) :: string?) or "unresolved"
		if world:get(entity, cts.Rendered) ~= staged then
			world:set(entity, cts.Rendered, staged)
			applyPreview(entity)
		end
	end
end

local liveHighlights: { [any]: Highlight } = {}

function preview.updateHighlights()
	local finished = world:has(cts.Finished, cts.Finished)
	for _, entity in conflicts.all() do
		local target = world:get(entity, cts.Target) :: Instance
		local spatial = target:IsDescendantOf(workspace) and target:IsA("PVInstance")
		if not spatial or finished then
			local existing = liveHighlights[entity]
			if existing then
				existing:Destroy()
				liveHighlights[entity] = nil
			end
			continue
		end

		local highlight = liveHighlights[entity]
		if not highlight then
			highlight = Instance.new("Highlight")
			highlight.Name = `RbxDiffHighlight_{world:get(entity, jecs.Name)}`
			highlight.Parent = target
			liveHighlights[entity] = highlight
		end

		local resolution = world:get(entity, cts.Resolution) :: string?

		-- Adorn whatever the preview is actually rendering: normally the
		-- target, but a staged edit side of a delete-vs-edit renders its
		-- swapped-in clone instead.
		local adornee: Instance = target
		if world:get(entity, cts.Kind) == "DeleteVsEdit" then
			local ours = world:get(entity, cts.Ours) :: SideInfo
			local theirs = world:get(entity, cts.Theirs) :: SideInfo
			local editSide = if ours.clone then "ours" else "theirs"
			local info = if editSide == "ours" then ours else theirs
			if resolution == editSide and info.clone then
				adornee = info.clone :: Instance
			end
		end
		highlight.Adornee = adornee

		local color = theme.statusColor(resolution)
		highlight.OutlineColor = color
		highlight.FillColor = color
		highlight.FillTransparency = 1 -- outline only
	end
end

return preview
