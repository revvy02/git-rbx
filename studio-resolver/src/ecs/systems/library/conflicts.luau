--[[
	Stateless helpers over conflict-entity world data — staging semantics and
	the custom-resolution predicates the UI and transport share. No caches, no
	module state: the world is the source of truth, these are just the verbs.
]]

local format = require("../../../library/format")

local cts = require("../../cts")
local world = require("../../world")

type SideInfo = cts.SideInfo
type ImpactSide = cts.ImpactSide
type ImpactOperation = cts.ImpactOperation

local conflicts = {}

-- The boot-sorted conflict entity array (immutable after boot).
function conflicts.list(): { any }
	return (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}
end

-- The display list: rigid-group entities + ungrouped conflicts, in order.
function conflicts.cards(): { any }
	return (world:get(cts.Cards, cts.Cards) :: { any }?) or conflicts.list()
end

-- Select a conflict without deciding or moving the camera. The logical id
-- keeps the corresponding virtual node selected across all four Explorer
-- projections; SelectedInstance continues to drive the property panel.
function conflicts.select(entity: any)
	world:set(cts.SelectedConflict, cts.SelectedConflict, entity)
	local target = world:get(entity, cts.Target) :: Instance?
	if not target then
		world:remove(cts.SelectedInstance, cts.SelectedInstance)
		world:remove(cts.SelectedExplorerId, cts.SelectedExplorerId)
		return
	end
	world:set(cts.SelectedInstance, cts.SelectedInstance, target)
	local subjectIds = (world:get(cts.VirtualSubjectIds, cts.VirtualSubjectIds) :: { [Instance]: number }?) or {}
	local id = subjectIds[target]
	if id then
		world:set(cts.SelectedExplorerId, cts.SelectedExplorerId, id)
	else
		world:remove(cts.SelectedExplorerId, cts.SelectedExplorerId)
	end
	local revealGeneration = (world:get(cts.ExplorerRevealGeneration, cts.ExplorerRevealGeneration) :: number?) or 0
	world:set(cts.ExplorerRevealGeneration, cts.ExplorerRevealGeneration, revealGeneration + 1)
end

-- Select one logical node from the direct patch while keeping the
-- owning conflict selected. Virtual nodes intentionally clear the property
-- panel but remain selectable/revealable in every version Explorer.
function conflicts.selectExplorerId(id: number)
	world:set(cts.SelectedExplorerId, cts.SelectedExplorerId, id)
	local subjects = (world:get(cts.VirtualSubjects, cts.VirtualSubjects) :: { [number]: Instance }?) or {}
	local subject = subjects[id]
	if subject then
		world:set(cts.SelectedInstance, cts.SelectedInstance, subject)
	else
		world:remove(cts.SelectedInstance, cts.SelectedInstance)
	end
	local generation = (world:get(cts.ExplorerRevealGeneration, cts.ExplorerRevealGeneration) :: number?) or 0
	world:set(cts.ExplorerRevealGeneration, cts.ExplorerRevealGeneration, generation + 1)
end

local function emptyImpact(): ImpactSide
	return {
		operations = {},
		affectedIds = {},
		instanceCount = 0,
		propertyCount = 0,
	}
end

-- One side's canonical decision patch. Rigid cards aggregate their member
-- patches while preserving direct operations and de-duplicating exact extent.
function conflicts.impact(entity: any, side: string): ImpactSide
	local component = if side == "ours" then cts.ImpactOurs else cts.ImpactTheirs
	local members = world:get(entity, cts.Members) :: { any }?
	if not members then
		return (world:get(entity, component) :: ImpactSide?) or emptyImpact()
	end

	local operations: { ImpactOperation } = {}
	local affected: { [number]: boolean } = {}
	local properties: { [string]: boolean } = {}
	for _, member in members do
		local impact = (world:get(member, component) :: ImpactSide?) or emptyImpact()
		for _, operation in impact.operations do
			table.insert(operations, operation)
			if operation.property then
				properties[`{operation.nodeId or 0}:{operation.property}`] = true
			end
		end
		for _, id in impact.affectedIds do
			affected[id] = true
		end
	end

	local affectedIds = {}
	for id in affected do
		table.insert(affectedIds, id)
	end
	table.sort(affectedIds)
	local propertyCount = 0
	for _ in properties do
		propertyCount += 1
	end
	return {
		operations = operations,
		affectedIds = affectedIds,
		instanceCount = #affectedIds,
		propertyCount = propertyCount,
	}
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

function conflicts.stageGroup(entity: any, side: string?)
	for _, member in (world:get(entity, cts.Members) :: { any }?) or {} do
		conflicts.stage(member, side)
	end
end

-- Group toggle: stage every member to the side, or unpick all when the
-- group is already uniformly on it.
function conflicts.chooseGroup(entity: any, side: string)
	conflicts.stageGroup(entity, if conflicts.groupResolution(entity) == side then nil else side)
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
-- normally the target, but clone-backed decisions render the selected clone.
-- Focus and highlights both follow the visible payload.
function conflicts.renderedInstance(entity: any): Instance
	local target = world:get(entity, cts.Target) :: Instance
	local kind = world:get(entity, cts.Kind)
	if kind == "PropertyBundle" then
		local side = world:get(entity, cts.Resolution) :: string?
		local info = if side == "ours"
			then world:get(entity, cts.Ours) :: SideInfo?
			elseif side == "theirs" then world:get(entity, cts.Theirs) :: SideInfo?
			else nil
		return if info and info.clone then info.clone else target
	elseif kind ~= "DeleteVsEdit" then
		return target
	end
	local keepSide = if conflicts.deleteSide(entity) == "ours" then "theirs" else "ours"
	if world:get(entity, cts.Resolution) == keepSide then
		local info = world:get(entity, if keepSide == "ours" then cts.Ours else cts.Theirs) :: SideInfo?
		if info and info.clone then
			return info.clone :: Instance
		end
		if info and info.moveOuts and info.moveOuts[1] then
			return info.moveOuts[1].snapshot
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
