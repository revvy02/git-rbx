--[[
	Component registry — every component/tag the resolver's world uses, in
	one place, after game-prototype's ecs/cts.luau. The diff layers will grow
	here: per-instance status tags, per-side value components, staging state.
]]

local packages = require("../packages")

local world = require("./world")

local jecs = packages.jecs

local cts = {
	-- The live preview Instance an entity mirrors.
	Ref = world:component(),
	-- A conflict entity's record (state.Conflict — shared table with
	-- state.conflicts, so staging mutations are visible through both).
	ConflictData = world:component(),
}

for name, ct in cts do
	world:set(ct, jecs.Name, name)
end

return cts
