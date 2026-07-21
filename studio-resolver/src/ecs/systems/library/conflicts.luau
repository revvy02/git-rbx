--[[
	Stateless helpers over conflict-entity world data — staging semantics and
	the custom-resolution predicates the UI and transport share. No caches, no
	module state: the world is the source of truth, these are just the verbs.
]]

local format = require("../../../library/format")

local cts = require("../../cts")
local world = require("../../world")

local conflicts = {}

-- The boot-sorted conflict entity array (immutable after boot).
function conflicts.list(): { any }
	return (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}
end

-- Staged decisions are session-local only and freely revisable; nothing is
-- written anywhere until Complete. stage() sets an explicit state (nil =
-- unpicked/base); choose() is the toggle the side buttons use.
function conflicts.stage(entity: any, side: string?)
	if world:has(cts.Completing, cts.Completing) or world:has(cts.Finished, cts.Finished) then
		return
	end
	if side then
		world:set(entity, cts.Resolution, side)
	else
		world:remove(entity, cts.Resolution)
	end
end

function conflicts.choose(entity: any, side: string)
	conflicts.stage(entity, if world:get(entity, cts.Resolution) == side then nil else side)
end

-- The live value of the conflicted property on the (copy) target instance.
-- nil when the property is serialized-only and Luau can't read it.
function conflicts.liveValue(entity: any): any
	local property = world:get(entity, cts.Property) :: string
	local target = world:get(entity, cts.Target) :: Instance
	if property == "Name" then
		return target.Name
	end
	local ok, value = format.readProperty(target, property)
	return if ok then value else nil
end

-- Custom = "edit the instance in Studio, that edit is the resolution".
-- Available for Property conflicts whose live value has an encodable type.
function conflicts.canCustom(entity: any): boolean
	return world:get(entity, cts.Kind) == "Property" and format.encodeValue(conflicts.liveValue(entity)) ~= nil
end

return conflicts
