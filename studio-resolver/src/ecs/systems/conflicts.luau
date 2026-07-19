--[[
	Conflict entities: one per conflict, carrying the shared record as
	ConflictData and ChildOf the contested instance's mirror entity.
]]

local packages = require("../../packages")

local state = require("../../state")

local cts = require("../cts")
local mirror = require("./mirror")
local world = require("../world")

local jecs = packages.jecs
local pair = jecs.pair

local conflicts = {}

function conflicts.boot()
	for _, conflict in state.conflicts do
		local entity = world:entity()
		world:set(entity, cts.ConflictData, conflict)
		world:add(entity, pair(jecs.ChildOf, mirror.entityFor(conflict.target)))
	end
end

return conflicts
