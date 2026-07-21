--[[
	Builds immutable Explorer trees on the four view singleton entities. Nodes
	are small Lua records, not ECS placement entities: canonical mirror identity
	is shared through `entity`/`subject`, while each projection owns its source,
	parent, children, and visibility. That permits moved and virtual occurrences
	without a multi-parent ECS graph or parallel relation/index bookkeeping.

	BASE/OURS/THEIRS are sparse overlays of one boot-time base snapshot and are
	built lazily when their tab is opened. RESULT follows the live mirror and is
	rebuilt when MirrorGeneration changes.
]]

local packages = require("../../packages")
local types = require("../types")

type SideInfo = types.SideInfo
type Base = types.Base
type ExplorerNode = types.ExplorerNode
type ExplorerProjection = types.ExplorerProjection

type Snapshot = {
	parent: { [Instance]: Instance },
	order: { [Instance]: number },
	instances: { Instance },
	entityOf: { [Instance]: any },
}

type ViewPlan = {
	source: { [Instance]: Instance },
	name: { [Instance]: string },
	deleted: { [Instance]: boolean },
	deepSource: { [Instance]: boolean },
	full: { [Instance]: boolean },
	parent: { [Instance]: Instance },
	hasParent: { [Instance]: boolean },
}

local function visibleChildren(inst: Instance): { Instance }
	local children = {}
	for _, child in inst:GetChildren() do
		if child.ClassName ~= "Highlight" and child.Name ~= "__RbxDiffMerge" then
			table.insert(children, child)
		end
	end
	table.sort(children, function(a: Instance, b: Instance)
		return if a.Name == b.Name then a.ClassName < b.ClassName else a.Name < b.Name
	end)
	return children
end

return function(world: types.World, cts: types.Cts, scheduler: types.Scheduler)
	local conflicts = (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}
	local sideViews = {
		base = cts.ExplorerBaseView,
		ours = cts.ExplorerOursView,
		theirs = cts.ExplorerTheirsView,
	}
	local snapshot: Snapshot? = nil
	local sideBuilt: { [string]: boolean } = {}
	local resultGeneration = -1
	local resultResolutionSignature = ""
	local generation = 0

	local function captureSnapshot(): Snapshot
		local excluded: { [Instance]: boolean } = {}
		for _, entity in conflicts do
			for _, info in
				{
					world:get(entity, cts.Ours) :: SideInfo?,
					world:get(entity, cts.Theirs) :: SideInfo?,
				}
			do
				if info and info.clone then
					excluded[info.clone] = true
				end
			end
		end

		local captured: Snapshot = {
			parent = {},
			order = {},
			instances = {},
			entityOf = (world:get(cts.EntityOf, cts.EntityOf) :: { [Instance]: any }?) or {},
		}
		local order = 0
		local function visit(entity: any, parent: Instance?)
			local inst = world:get(entity, cts.Ref) :: Instance?
			if not inst or excluded[inst] then
				return
			end
			order += 1
			captured.order[inst] = order
			table.insert(captured.instances, inst)
			if parent then
				captured.parent[inst] = parent
			end
			for _, child in (world:get(entity, cts.Children) :: { any }?) or {} do
				visit(child, inst)
			end
		end
		for _, root in (world:get(cts.Roots, cts.Roots) :: { any }?) or {} do
			visit(root, nil)
		end
		return captured
	end

	local function planFor(tab: string): ViewPlan
		local plan: ViewPlan = {
			source = {},
			name = {},
			deleted = {},
			deepSource = {},
			full = {},
			parent = {},
			hasParent = {},
		}
		for _, entity in conflicts do
			local target = world:get(entity, cts.Target) :: Instance
			local kind = world:get(entity, cts.Kind)
			local base = world:get(entity, cts.Base) :: Base
			local info = if tab == "ours"
				then world:get(entity, cts.Ours) :: SideInfo
				elseif tab == "theirs" then world:get(entity, cts.Theirs) :: SideInfo
				else nil

			if info and info.deleted then
				plan.deleted[target] = true
			else
				plan.full[target] = true
				if info and info.clone then
					plan.source[target] = info.clone
					plan.deepSource[target] = kind == "DeleteVsEdit"
				end
			end
			if tab == "base" and kind == "Property" and world:get(entity, cts.Property) == "Name" then
				plan.name[target] = base.value :: string
			end
			if kind == "MoveTarget" then
				plan.hasParent[target] = true
				local parent = if info then info.destination or base.parent else base.parent
				if parent then
					plan.parent[target] = parent
				end
			end
		end
		return plan
	end

	local function buildSide(tab: string, captured: Snapshot): ExplorerProjection
		local plan = planFor(tab)
		local parents: { [Instance]: Instance } = {}
		local children: { [Instance]: { Instance } } = {}
		local roots = {}
		for _, inst in captured.instances do
			local parent = if plan.hasParent[inst] then plan.parent[inst] else captured.parent[inst]
			if parent then
				parents[inst] = parent
				children[parent] = children[parent] or {}
				table.insert(children[parent], inst)
			else
				table.insert(roots, inst)
			end
		end

		local function sort(instances: { Instance })
			table.sort(instances, function(a: Instance, b: Instance)
				local aSource, bSource = plan.source[a] or a, plan.source[b] or b
				return if aSource.Name == bSource.Name
					then (captured.order[a] or math.huge) < (captured.order[b] or math.huge)
					else aSource.Name < bSource.Name
			end)
		end
		for _, list in children do
			sort(list)
		end
		sort(roots)

		local needed: { [Instance]: boolean } = {}
		local function markChain(inst: Instance?)
			local seen = {}
			while inst and not seen[inst] do
				seen[inst] = true
				needed[inst] = true
				inst = parents[inst]
			end
		end
		for _, entity in conflicts do
			local target = world:get(entity, cts.Target) :: Instance
			needed[target] = not plan.deleted[target]
			markChain(parents[target])
		end

		local projection: ExplorerProjection = { roots = {}, index = {} }
		local function nodeFor(source: Instance?, subject: Instance?, displayName: string?): ExplorerNode
			local display = source or subject
			local node: ExplorerNode = {
				source = source,
				subject = subject,
				entity = if subject then captured.entityOf[subject] else nil,
				name = displayName or if display then display.Name else "(virtual)",
				className = if display then display.ClassName else nil,
				children = {},
			}
			if source then
				projection.index[source] = node
			end
			if subject then
				projection.index[subject] = node
			end
			return node
		end
		local function attach(node: ExplorerNode, parent: ExplorerNode?)
			node.parent = parent
			table.insert(if parent then parent.children else projection.roots, node)
		end
		local function copySource(source: Instance, parent: ExplorerNode)
			for _, child in visibleChildren(source) do
				local node = nodeFor(child, nil, nil)
				attach(node, parent)
				copySource(child, node)
			end
		end

		local visiting: { [Instance]: boolean } = {}
		local function visit(inst: Instance, parent: ExplorerNode?, inheritedFull: boolean)
			if plan.deleted[inst] or visiting[inst] then
				return
			end
			local full = inheritedFull or plan.full[inst] == true
			if not full and not needed[inst] then
				return
			end
			visiting[inst] = true
			local source = plan.source[inst] or inst
			local node = nodeFor(source, inst, plan.name[inst])
			attach(node, parent)
			if plan.deepSource[inst] then
				copySource(source, node)
			else
				for _, child in children[inst] or {} do
					visit(child, node, full)
				end
			end
			visiting[inst] = nil
		end
		for _, root in roots do
			visit(root, nil, false)
		end
		return projection
	end

	local function buildResult(): ExplorerProjection
		local projection: ExplorerProjection = { roots = {}, index = {} }
		-- Delete-vs-edit previews cannot actually remove the base target: it is
		-- retained invisibly so the choice can be toggled without rebuilding the
		-- imported file. When the delete side is staged, or an edited keep clone
		-- is swapped in, that hidden target is an implementation detail and must
		-- not appear as a duplicate in the logical RESULT tree.
		local hiddenRoots: { [Instance]: boolean } = {}
		for _, conflict in conflicts do
			if world:get(conflict, cts.Kind) == "DeleteVsEdit" then
				local staged = world:get(conflict, cts.Resolution) :: string?
				if staged then
					local ours = world:get(conflict, cts.Ours) :: SideInfo
					local theirs = world:get(conflict, cts.Theirs) :: SideInfo
					local deleteSide = if ours.deleted then "ours" elseif theirs.deleted then "theirs" else nil
					local keepSide = if deleteSide == "ours" then "theirs" else "ours"
					local keepInfo = if keepSide == "ours" then ours else theirs
					if staged == deleteSide or (staged == keepSide and keepInfo.clone ~= nil) then
						hiddenRoots[world:get(conflict, cts.Target) :: Instance] = true
					end
				end
			end
		end
		local function visit(entity: any, parent: ExplorerNode?): ExplorerNode?
			local inst = world:get(entity, cts.Ref) :: Instance?
			if not inst or hiddenRoots[inst] then
				return nil
			end
			local node: ExplorerNode = {
				source = inst,
				subject = inst,
				entity = entity,
				name = inst.Name,
				className = inst.ClassName,
				parent = parent,
				children = {},
			}
			projection.index[inst] = node
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
			snapshot = snapshot or captureSnapshot()
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
				world:set(view, cts.ExplorerProjection, buildSide(tab, snapshot :: Snapshot))
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
