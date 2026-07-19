--[[
	Schedules the instance mirror's frame diff (library/mirror.luau).
]]

local packages = require("../../packages")

local mirror = require("../library/mirror")
local types = require("../types")

return function(_world: types.World, _cts: types.Cts, scheduler: types.Scheduler)
	scheduler:addSystem({
		phase = packages.planckRunService.Phases.Heartbeat,
		system = mirror.sync,
	})
end
