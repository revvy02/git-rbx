--[[
	Conflict entities — the source of truth for the merge. boot() turns the
	stamped container entries into entities with granular components (Kind,
	Target, Sides, Resolution, ...), each ChildOf the contested instance's
	mirror entity. Staging mutates Resolution through the world; absence of
	Resolution = unpicked (base).
]]

local packages = require("../../packages")

local format = require("../../format")

local cts = require("../cts")
local mirror = require("./mirror")
local world = require("../world")

local jecs = packages.jecs
local pair = jecs.pair

-- One parsed container entry, as init.luau reads them from the DOM.
export type ConflictRecord = {
	name: string,
	kind: string,
	path: string,
	property: string?,
	resolved: string?,
	target: Instance,
	entry: Instance,
}

local conflicts = {}

function conflicts.boot(records: { ConflictRecord })
	for index, record in records do
		local entity = world:entity()
		world:set(entity, jecs.Name, record.name)
		world:set(entity, cts.Order, index)
		world:set(entity, cts.Kind, record.kind)
		world:set(entity, cts.Path, record.path)
		world:set(entity, cts.Target, record.target)
		world:set(entity, cts.Entry, record.entry)
		if record.property then
			world:set(entity, cts.Property, record.property)
		end
		if record.resolved then
			world:set(entity, cts.Resolution, record.resolved)
		end
		world:add(entity, pair(jecs.ChildOf, mirror.entityFor(record.target)))
	end
end

-- Every conflict entity in stable list order.
function conflicts.all(): { any }
	local out: { any } = {}
	for entity in world:query(cts.Order):iter() do
		table.insert(out, entity)
	end
	table.sort(out, function(a: any, b: any)
		return (world:get(a, cts.Order) :: number) < (world:get(b, cts.Order) :: number)
	end)
	return out
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
function conflicts.liveValue(entity: any): any
	local property = world:get(entity, cts.Property) :: string
	local target = world:get(entity, cts.Target) :: Instance
	if property == "Name" then
		return target.Name
	end
	return (target :: any)[property]
end

-- Custom = "edit the instance in Studio, that edit is the resolution".
-- Available for Property conflicts whose live value has an encodable type.
function conflicts.canCustom(entity: any): boolean
	return world:get(entity, cts.Kind) == "Property" and format.encodeValue(conflicts.liveValue(entity)) ~= nil
end

return conflicts
