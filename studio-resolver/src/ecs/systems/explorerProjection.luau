--[[
	Builds immutable Explorer trees on the four view singleton entities.

	BASE/OURS/THEIRS come from complete virtual DOM manifests captured by Rust
	before the merge mutates anything. A logical node id is shared by matched
	instances across versions, while each manifest independently owns existence,
	parentage, order, name, and class. Concrete Instances are optional backing
	subjects; conflict snapshots replace them where exact branch data exists.

	RESULT follows the live mirror because staged decisions change the preview.
	Explorer hierarchy deliberately remains plain data rather than ECS ChildOf:
	one logical node can have three different parents or be absent in a version.
]]

local packages = require("../../packages")
local types = require("../types")

type SideInfo = types.SideInfo
type ExplorerNode = types.ExplorerNode
type ExplorerProjection = types.ExplorerProjection
type VersionTreeNode = types.VersionTreeNode
type VersionTrees = types.VersionTrees

local function visibleChildren(inst: Instance): { Instance }
	local children = {}
	for _, child in inst:GetChildren() do
		if child.ClassName ~= "Highlight" and child.Name ~= "__GitRbxConflicts" then
			table.insert(children, child)
		end
	end
	return children
end

local function sourceKey(name: string, className: string): string
	return `{className}\0{name}`
end

return function(world: types.World, cts: types.Cts, scheduler: types.Scheduler)
	local conflicts = (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}
	local versionTrees = (world:get(cts.VersionTrees, cts.VersionTrees) :: VersionTrees?) or {}
	local subjects = (world:get(cts.VirtualSubjects, cts.VirtualSubjects) :: { [number]: Instance }?) or {}
	local subjectIds = (world:get(cts.VirtualSubjectIds, cts.VirtualSubjectIds) :: { [Instance]: number }?) or {}
	local baseParents: { [number]: number } = {}
	for _, record in versionTrees.base or {} do
		baseParents[record.id] = record.parent or 0
	end
	local versionChildren: { [string]: { [number]: { VersionTreeNode } } } = {}
	for side, records in versionTrees do
		local children: { [number]: { VersionTreeNode } } = {}
		for _, record in records do
			if record.parent then
				local bucket = children[record.parent] or {}
				children[record.parent] = bucket
				table.insert(bucket, record)
			end
		end
		versionChildren[side] = children
	end
	local sideViews = {
		base = cts.ExplorerBaseView,
		ours = cts.ExplorerOursView,
		theirs = cts.ExplorerTheirsView,
	}
	local sideBuilt: { [string]: boolean } = {}
	local resultGeneration = -1
	local resultResolutionSignature = ""
	local generation = 0

	local function finishSelectionIndex(projection: ExplorerProjection)
		local idIndex = projection.idIndex or {}
		local selectionIndex: { [number]: ExplorerNode } = {}
		local resolved: { [number]: boolean } = {}
		for id, node in idIndex do
			selectionIndex[id] = node
			resolved[id] = true
		end

		local function nearest(id: number): ExplorerNode?
			if resolved[id] then
				return selectionIndex[id]
			end
			resolved[id] = true
			local parent = baseParents[id]
			if parent and parent ~= 0 then
				selectionIndex[id] = nearest(parent)
			end
			return selectionIndex[id]
		end
		for id in baseParents do
			nearest(id)
		end
		projection.selectionIndex = selectionIndex
	end

	local function bindSource(projection: ExplorerProjection, node: ExplorerNode, source: Instance, deep: boolean)
		node.source = source
		projection.index[source] = node
		if not deep then
			return
		end

		-- A delete-vs-edit clone contains the complete branch subtree. Match
		-- duplicate-name children by occurrence within their exact class/name
		-- bucket; the virtual manifest remains authoritative for topology.
		local buckets: { [string]: { Instance } } = {}
		for _, child in visibleChildren(source) do
			local key = sourceKey(child.Name, child.ClassName)
			local bucket = buckets[key] or {}
			buckets[key] = bucket
			table.insert(bucket, child)
		end
		local consumed: { [string]: number } = {}
		for _, childNode in node.children do
			local key = sourceKey(childNode.name, childNode.className or "")
			local index = (consumed[key] or 0) + 1
			consumed[key] = index
			local child = buckets[key] and buckets[key][index]
			if child then
				bindSource(projection, childNode, child, true)
			end
		end
	end

	local function buildVersion(tab: string): ExplorerProjection?
		local records = versionTrees[tab]
		if not records then
			return nil
		end
		local entityOf = (world:get(cts.EntityOf, cts.EntityOf) :: { [Instance]: any }?) or {}
		local projection: ExplorerProjection = { roots = {}, index = {}, idIndex = {}, selectionIndex = {} }
		local idIndex = projection.idIndex :: { [number]: ExplorerNode }

		for _, record: VersionTreeNode in records do
			local subject = subjects[record.id]
			local node: ExplorerNode = {
				id = record.id,
				source = subject,
				subject = subject,
				entity = if subject then entityOf[subject] else nil,
				name = record.name,
				className = record.className,
				children = {},
			}
			idIndex[record.id] = node
			if subject then
				projection.index[subject] = node
			end
		end

		for _, record: VersionTreeNode in records do
			local node = idIndex[record.id]
			local parent = record.parent and idIndex[record.parent]
			node.parent = parent
			table.insert(if parent then parent.children else projection.roots, node)
		end

		local sideComponent = if tab == "ours" then cts.Ours elseif tab == "theirs" then cts.Theirs else nil
		if sideComponent then
			for _, conflict in conflicts do
				local target = world:get(conflict, cts.Target) :: Instance
				local node = projection.index[target]
				local info = world:get(conflict, sideComponent) :: SideInfo?
				if node and info and info.clone then
					bindSource(projection, node, info.clone, world:get(conflict, cts.Kind) == "DeleteVsEdit")
				end
			end
		end
		finishSelectionIndex(projection)

		return projection
	end

	local function buildResult(): ExplorerProjection
		local projection: ExplorerProjection = { roots = {}, index = {}, idIndex = {}, selectionIndex = {} }
		-- Clone-backed previews retain the base target invisibly so decisions can
		-- be toggled. Do not expose that implementation target beside its clone.
		local hiddenRoots: { [Instance]: boolean } = {}
		for _, conflict in conflicts do
			local kind = world:get(conflict, cts.Kind)
			if kind == "DeleteVsEdit" then
				local staged = world:get(conflict, cts.Resolution) :: string?
				if staged then
					local ours = world:get(conflict, cts.Ours) :: SideInfo
					local theirs = world:get(conflict, cts.Theirs) :: SideInfo
					local deleteSide = if ours.deleted then "ours" elseif theirs.deleted then "theirs" else nil
					local keepSide = if deleteSide == "ours" then "theirs" else "ours"
					local keepInfo = if keepSide == "ours" then ours else theirs
					if
						staged == deleteSide
						or (staged == keepSide and (keepInfo.clone ~= nil or #(keepInfo.moveOuts or {}) > 0))
					then
						hiddenRoots[world:get(conflict, cts.Target) :: Instance] = true
					end
				end
			elseif kind == "PropertyBundle" then
				local staged = world:get(conflict, cts.Resolution) :: string?
				local info = if staged == "ours"
					then world:get(conflict, cts.Ours) :: SideInfo
					elseif staged == "theirs" then world:get(conflict, cts.Theirs) :: SideInfo
					else nil
				if info and info.clone then
					hiddenRoots[world:get(conflict, cts.Target) :: Instance] = true
				end
			end
		end

		local cloneIds: { [Instance]: number } = {}
		local function bindCloneIds(source: Instance, id: number, side: string)
			cloneIds[source] = id
			local childRecords = versionChildren[side] and versionChildren[side][id]
			if not childRecords then
				return
			end
			local buckets: { [string]: { Instance } } = {}
			for _, child in visibleChildren(source) do
				local key = sourceKey(child.Name, child.ClassName)
				local bucket = buckets[key] or {}
				buckets[key] = bucket
				table.insert(bucket, child)
			end
			local consumed: { [string]: number } = {}
			for _, record in childRecords do
				local key = sourceKey(record.name, record.className)
				local index = (consumed[key] or 0) + 1
				consumed[key] = index
				local child = buckets[key] and buckets[key][index]
				if child then
					bindCloneIds(child, record.id, side)
				end
			end
		end
		for _, conflict in conflicts do
			local id = subjectIds[world:get(conflict, cts.Target) :: Instance]
			if id then
				local ours = world:get(conflict, cts.Ours) :: SideInfo?
				local theirs = world:get(conflict, cts.Theirs) :: SideInfo?
				if ours and ours.clone then
					bindCloneIds(ours.clone, id, "ours")
				end
				if theirs and theirs.clone then
					bindCloneIds(theirs.clone, id, "theirs")
				end
			end
		end

		local idIndex = projection.idIndex :: { [number]: ExplorerNode }
		local function visit(entity: any, parent: ExplorerNode?): ExplorerNode?
			local inst = world:get(entity, cts.Ref) :: Instance?
			if not inst or hiddenRoots[inst] then
				return nil
			end
			local id = subjectIds[inst] or cloneIds[inst]
			local node: ExplorerNode = {
				id = id,
				source = inst,
				subject = inst,
				entity = entity,
				name = inst.Name,
				className = inst.ClassName,
				parent = parent,
				children = {},
			}
			projection.index[inst] = node
			if id then
				idIndex[id] = node
			end
			for _, child in (world:get(entity, cts.Children) :: { any }?) or {} do
				local childNode = visit(child, node)
				if childNode then
					table.insert(node.children, childNode)
				end
			end
			return node
		end
		for _, root in (world:get(cts.Roots, cts.Roots) :: { any }?) or {} do
			local node = visit(root, nil)
			if node then
				table.insert(projection.roots, node)
			end
		end
		finishSelectionIndex(projection)
		return projection
	end

	local function resolutionSignature(): string
		local states = table.create(#conflicts)
		for index, conflict in conflicts do
			states[index] = (world:get(conflict, cts.Resolution) :: string?) or "unresolved"
		end
		return table.concat(states, "|")
	end

	scheduler:addSystem({
		name = "explorer_projection",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			if world:has(cts.Finished, cts.Finished) then
				return
			end
			local mirrorGeneration = world:get(cts.MirrorGeneration, cts.MirrorGeneration) :: number?
			if not mirrorGeneration then
				return
			end

			local changed = false
			local signature = resolutionSignature()
			if resultGeneration ~= mirrorGeneration or resultResolutionSignature ~= signature then
				resultGeneration = mirrorGeneration
				resultResolutionSignature = signature
				world:set(cts.ExplorerResultView, cts.ExplorerProjection, buildResult())
				changed = true
			end
			local tab = world:get(cts.ViewTab, cts.ViewTab) :: string
			local view = sideViews[tab]
			if view and not sideBuilt[tab] then
				-- Version-1 artifacts had no complete manifests. Showing the full
				-- live tree is a truthful completeness fallback, though not an exact
				-- historical version; newly generated artifacts always use manifests.
				world:set(view, cts.ExplorerProjection, buildVersion(tab) or buildResult())
				sideBuilt[tab] = true
				changed = true
			end
			if changed then
				generation += 1
				world:set(cts.ExplorerGeneration, cts.ExplorerGeneration, generation)
			end
		end,
	})
end
