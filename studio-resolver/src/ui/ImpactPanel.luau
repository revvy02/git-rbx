--[[
	Direct conflict patch for the selected conflict's staged (or default)
	side, rendered through the shared PatchVisualizer.

	This module only adapts: Rust's per-side impact operations plus the
	version manifests become a neutral Patch. Property edits are Edit rows,
	a Reparent is a `Parent` row, an added subtree lists every added
	descendant (as Rojo does), a deleted subtree is one Remove row, and a
	pivot conflict contributes a `Pivot` row carrying the side's delta.
]]

local Element = require("../packages").miu.Element

local PatchVisualizer = require("./PatchVisualizer")
local conflicts = require("../ecs/systems/library/conflicts")
local cts = require("../ecs/cts")
local format = require("../library/format")
local world = require("../ecs/world")

type ImpactSide = cts.ImpactSide
type SideInfo = cts.SideInfo
type VersionTreeNode = cts.VersionTreeNode
type VersionTrees = cts.VersionTrees
type Patch = PatchVisualizer.Patch
type PatchNodeInput = PatchVisualizer.PatchNodeInput

local HEADER_HEIGHT = 24

local indexedTrees: VersionTrees? = nil
local recordsBySide: { [string]: { [number]: VersionTreeNode } } = {}
local childrenBySide: { [string]: { [number]: { number } } } = {}

local function indexes()
	local trees = (world:get(cts.VersionTrees, cts.VersionTrees) :: VersionTrees?) or {}
	if indexedTrees == trees then
		return recordsBySide, childrenBySide
	end
	indexedTrees = trees
	recordsBySide = {}
	childrenBySide = {}
	for side, records in trees do
		local index: { [number]: VersionTreeNode } = {}
		local children: { [number]: { number } } = {}
		for _, record in records do
			index[record.id] = record
			if record.parent then
				local list = children[record.parent]
				if not list then
					list = {}
					children[record.parent] = list
				end
				table.insert(list, record.id)
			end
		end
		recordsBySide[side] = index
		childrenBySide[side] = children
	end
	return recordsBySide, childrenBySide
end

local function buildPatch(entity: any, side: string): (Patch, ImpactSide)
	local impact = conflicts.impact(entity, side)
	local records, children = indexes()
	local nodes: { [number]: PatchNodeInput } = {}

	local function record(id: number, version: string): VersionTreeNode?
		local own = records[version]
		return (own and own[id]) or (records.base and records.base[id])
	end

	-- Materialize a node and the ancestor chain that reaches it, from the
	-- version the operation lives in (base for deletes, the side otherwise).
	local function ensure(id: number, version: string): PatchNodeInput?
		local existing = nodes[id]
		if existing then
			return existing
		end
		local found = record(id, version)
		if not found then
			return nil
		end
		local node: PatchNodeInput = {
			id = id,
			name = found.name,
			className = found.className,
			parent = found.parent,
			rows = {},
		}
		nodes[id] = node
		if found.parent then
			ensure(found.parent, version)
		end
		return node
	end

	local function addSubtree(id: number, version: string)
		local list = children[version] and children[version][id]
		for _, childId in list or {} do
			local child = ensure(childId, version)
			if child then
				child.kind = "Add"
				addSubtree(childId, version)
			end
		end
	end

	for _, operation in impact.operations do
		if not operation.nodeId then
			continue
		end
		local version = if operation.kind == "Delete" then "base" else side
		local node = ensure(operation.nodeId, version)
		if not node then
			continue
		end
		local rows = node.rows :: { PatchVisualizer.ChangeRow }
		if operation.kind == "Property" then
			table.insert(rows, {
				property = operation.property or "Property",
				kind = "Edit",
				current = operation.before,
				incoming = operation.after,
			})
		elseif operation.kind == "Reparent" then
			table.insert(rows, {
				property = "Parent",
				kind = "Reparent",
				currentText = format.escapeRich(operation.path),
				incomingText = format.escapeRich(operation.destination or "?"),
			})
		elseif operation.kind == "Add" then
			node.kind = "Add"
			addSubtree(operation.nodeId, version)
		elseif operation.kind == "Delete" then
			node.kind = "Remove"
			node.subtreeCount = operation.instanceCount
		end
	end

	-- Pivot conflicts carry no edit operations; the decision is the delta.
	local info = world:get(entity, if side == "ours" then cts.Ours else cts.Theirs) :: SideInfo?
	if info and info.delta then
		local target = world:get(entity, cts.Target) :: Instance?
		local ids = world:get(cts.VirtualSubjectIds, cts.VirtualSubjectIds) :: { [Instance]: number }?
		local id = if target and ids then ids[target] else nil
		local node = if id then ensure(id, "base") else nil
		if node then
			node.kind = node.kind or "Pivot"
			table.insert(node.rows :: { PatchVisualizer.ChangeRow }, {
				property = "Pivot",
				kind = "Pivot",
				currentText = "base placement",
				incomingText = format.escapeRich(format.formatDelta(info.delta)),
			})
		end
	end

	return { nodes = nodes }, impact
end

local function resolutionOf(entity: any): string?
	return if world:get(entity, cts.Members)
		then conflicts.groupResolution(entity)
		else world:get(entity, cts.Resolution) :: string?
end

local function ImpactPanel()
	local entity = world:get(cts.SelectedConflict, cts.SelectedConflict)
	local resolution = entity and resolutionOf(entity) or nil
	local side = if resolution == "theirs" then "theirs" else "ours"

	local patch: Patch = { nodes = {} }
	local impact: ImpactSide? = nil
	if entity then
		patch, impact = buildPatch(entity, side)
	end

	Element("Frame", {
		id = "impact",
		Size = UDim2.fromScale(1, 1),
		tags = { "surface", "vs-border" },
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = if impact
				then `DIRECT DIFF — {impact.instanceCount} CHANGED · {impact.propertyCount} {if impact.propertyCount
						== 1
					then "PROPERTY"
					else "PROPERTIES"}`
				else "DIRECT DIFF",
			Size = UDim2.new(1, 0, 0, HEADER_HEIGHT),
			tags = { "header-band" },
		})

		PatchVisualizer({
			patch = patch,
			stateKey = if entity then `{tostring(entity)}:{side}` else nil,
			headers = { "Property", "Base", string.upper(side:sub(1, 1)) .. side:sub(2) },
			selectedId = world:get(cts.SelectedExplorerId, cts.SelectedExplorerId) :: number?,
			onSelect = function(node)
				conflicts.selectExplorerId(node.id :: number)
			end,
			emptyText = "No direct changes on this side",
			Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT),
			Position = UDim2.fromOffset(0, HEADER_HEIGHT),
		})
	end)
end

return ImpactPanel
