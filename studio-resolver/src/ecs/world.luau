--[[
	The jecs world, and nothing else — components live in ecs/cts.luau,
	behaviors in ecs/systems/. Structured after game-prototype's shared/ecs.
]]

local packages = require("../packages")

return packages.jecs.world(true)
