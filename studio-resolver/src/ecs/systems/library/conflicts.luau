--[[
	Stateless helpers over conflict-entity world data — staging semantics and
	the custom-resolution predicates the UI and transport share. No caches, no
	module state: the world is the source of truth, these are just the verbs.
]]

local format = require("../../../library/format")

local cts = require("../../cts")
local world = require("../../world")

type SideInfo = cts.SideInfo

local conflicts = {}

-- The boot-sorted conflict entity array (immutable after boot).
function conflicts.list(): { any }
	return (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}
end

-- The display list: rigid-group entities + ungrouped conflicts, in order.
function conflicts.cards(): { any }
	return (world:get(cts.Cards, cts.Cards) :: { any }?) or conflicts.list()
end

-- A group's uniform member resolution, or nil when unpicked/mixed.
function conflicts.groupResolution(entity: any): string?
	local members = (world:get(entity, cts.Members) :: { any }?) or {}
	local first = if members[1] then world:get(members[1], cts.Resolution) :: string? else nil
	if not first then
		return nil
	end
	for index = 2, #members do
		if world:get(members[index], cts.Resolution) ~= first then
			return nil
		end
	end
	return first
end

-- Group toggle: stage every member to the side, or unpick all when the
-- group is already uniformly on it.
function conflicts.chooseGroup(entity: any, side: string)
	local next = if conflicts.groupResolution(entity) == side then nil else side
	for _, member in (world:get(entity, cts.Members) :: { any }?) or {} do
		conflicts.stage(member, next)
	end
end

-- Delete-vs-edit: which side deletes, from the stamped Deleted attributes.
-- The other side keeps — via an edited clone or a move out of the subtree.
function conflicts.deleteSide(entity: any): string?
	local ours = world:get(entity, cts.Ours) :: SideInfo?
	local theirs = world:get(entity, cts.Theirs) :: SideInfo?
	if ours and ours.deleted then
		return "ours"
	elseif theirs and theirs.deleted then
		return "theirs"
	end
	return nil
end

-- What the 3D preview is actually rendering for this conflict right now:
-- normally the target, but a staged keep side with an edited clone renders
-- the clone instead (it carries its own true coordinates). Focus and
-- highlights both follow this.
function conflicts.renderedInstance(entity: any): Instance
	local target = world:get(entity, cts.Target) :: Instance
	if world:get(entity, cts.Kind) ~= "DeleteVsEdit" then
		return target
	end
	local keepSide = if conflicts.deleteSide(entity) == "ours" then "theirs" else "ours"
	if world:get(entity, cts.Resolution) == keepSide then
		local info = world:get(entity, if keepSide == "ours" then cts.Ours else cts.Theirs) :: SideInfo?
		if info and info.clone then
			return info.clone :: Instance
		end
	end
	return target
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
