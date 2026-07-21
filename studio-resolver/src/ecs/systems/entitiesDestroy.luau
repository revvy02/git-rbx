--[[
	End-of-frame cleanup sweep, after game-prototype's core/entitiesDestroy:
	anything tagged Destroy is deleted in one batch. Registered last so
	requests consumed earlier in the frame don't live past it.
]]

local packages = require("../../packages")

local types = require("../types")

return function(world: types.World, cts: types.Cts, scheduler: types.Scheduler)
	scheduler:addSystem({
		name = "entities_destroy",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			for entity in world:each(cts.Destroy) do
				world:delete(entity)
			end
		end,
	})
end
