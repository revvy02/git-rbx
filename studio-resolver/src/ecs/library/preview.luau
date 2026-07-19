--[[
	Preview boot + camera. boot() stashes the stamped container out of the
	scene and fills each conflict entity's side info (Ours/Theirs folders)
	and Base snapshot (what unpicking restores). focusOn() frames a conflict
	target in view. The per-frame render/highlight passes live in systems/.
]]

local conflicts = require("./conflicts")
local cts = require("../cts")
local world = require("../world")

local getBoundingBox = require("../../viewport/getBoundingBox")
local getIdealCameraCFrame = require("../../viewport/getIdealCameraCFrame")

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

return preview
