--[[
	Camera focus as request entities: the View button spawns an entity with a
	FocusRequest payload (the conflict entity to frame); this system consumes
	every pending request and tags it Destroy for the cleanup sweep.

	Framing is a fitted distance from the target's bounding box and the live
	camera FOV/aspect (vendored miu-viewport math). Focus is the edit
	camera's orbit pivot: it must be written together with the CFrame, or
	Studio keeps steering around the stale pivot and the view drifts off
	target.
]]

local packages = require("../../packages")

local conflicts = require("./library/conflicts")
local types = require("../types")

local getBoundingBox = require("../../library/viewport/getBoundingBox")
local getIdealCameraCFrame = require("../../library/viewport/getIdealCameraCFrame")

return function(world: types.World, cts: types.Cts, scheduler: types.Scheduler)
	local function focusOn(conflictEntity: any)
		local camera = workspace.CurrentCamera
		-- Frame what the preview is actually rendering — the staged clone
		-- when a delete-vs-edit keep side is staged, the target otherwise.
		local target = conflicts.renderedInstance(conflictEntity)
		local cframe, size = getBoundingBox(target)
		camera.CFrame = getIdealCameraCFrame(cframe, size, {
			fov = camera.FieldOfView,
			aspectRatio = camera.ViewportSize.X / camera.ViewportSize.Y,
		})
		camera.Focus = cframe
	end

	local focusRequests = world:query(cts.FocusRequest):cached()

	scheduler:addSystem({
		name = "focus_camera",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			for requestEntity, conflictEntity in focusRequests do
				focusOn(conflictEntity)
				world:add(requestEntity, cts.Destroy)
			end
		end,
	})
end
