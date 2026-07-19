--[[
	The planck scheduler over the world, RunService-phased — after
	game-prototype's ecs/scheduler.luau (minus their bench/jabby plugins).
	Systems register in init.luau and run as soon as they're added
	(stock planck; game-prototype's pause-until-start comes from their patch).
]]

local packages = require("../packages")

local world = require("./world")

local Planck = packages.planck
local PlanckRunService = packages.planckRunService

local scheduler = Planck.Scheduler.new(world):addPlugin(PlanckRunService.Plugin.new())

return scheduler
