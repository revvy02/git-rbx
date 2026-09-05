--[[
	Plain explorer over the merge's four states, tabbed BASE | OURS | THEIRS
	| RESULT. No diff presentation here yet (badges, tints, phantoms) — this
	is a normal explorer per state; the diff layers come back iteratively.

	All four tabs walk ECS-owned Explorer projection tables. Nodes share
	canonical subjects but carry view-specific sources and parents, so reparented,
	deleted, cloned, or virtualized content needs no UI-side tree grafting.

	Conflict ancestor chains start expanded so each contested root is visible,
	but subtree contents start collapsed. Search temporarily forces every branch
	open so hidden descendants remain discoverable, then shows each name/class
	match with its visual ancestor chain.
]]

local packages = require("@shared/packages")
local Element = packages.miu.Element
local VirtualList = packages.virtualList.VirtualList
local Dimensions = packages.virtualList.Dimensions

local TreeRow = require("@self/TreeRow")

local format = require("@shared/library/format")
local theme = require("@shared/library/theme")

local cts = require("../ecs/cts")
local world = require("../ecs/world")

type ExplorerNode = cts.ExplorerNode
type ExplorerProjection = cts.ExplorerProjection

type Row = {
	node: ExplorerNode,
	source: Instance?,
	subject: Instance?,
	expansionKey: any,
	label: string,
	searchText: string,
	className: string?,
	phantom: boolean?,
	depth: number,
	hasChildren: boolean,
	isExpanded: boolean,
}

local VIEW_ENTITIES = {
	base = cts.ExplorerBaseView,
	ours = cts.ExplorerOursView,
	theirs = cts.ExplorerTheirsView,
	result = cts.ExplorerResultView,
}

local expanded: { [any]: boolean } = {}
local expansionGen = 0
local seededViews: { [any]: boolean } = {}
local lastRevealedInstance: Instance? = nil
local lastRevealedId: number? = nil
local lastRevealedView: any? = nil
local lastRevealedGeneration = -1
local lastRevealRequest = -1
local pendingCenterNode: ExplorerNode? = nil
local pendingCenterView: any? = nil
local searchText = ""
local searchGen = 0

local function setExpanded(key: any, isExpanded: boolean)
	expanded[key] = isExpanded
	expansionGen += 1
end

local function expansionKey(node: ExplorerNode): any
	return if node.id then `virtual:{node.id}` else node.subject or node.source or node
end

local function selectedNode(projection: ExplorerProjection): ExplorerNode?
	local id = world:get(cts.SelectedExplorerId, cts.SelectedExplorerId) :: number?
	local node = id and projection.selectionIndex and projection.selectionIndex[id]
	if node then
		return node
	end
	local inst = world:get(cts.SelectedInstance, cts.SelectedInstance) :: Instance?
	return inst and projection.index[inst]
end

local function seedExpansion(view: any, projection: ExplorerProjection)
	if seededViews[view] then
		return
	end
	seededViews[view] = true
	expansionGen += 1
	local subjectIds = (world:get(cts.VirtualSubjectIds, cts.VirtualSubjectIds) :: { [Instance]: number }?) or {}
	for _, target in world:query(cts.Target) do
		local inst = target :: Instance
		local id = subjectIds[inst]
		local node = (id and projection.selectionIndex and projection.selectionIndex[id]) or projection.index[inst]
		node = node and node.parent
		while node do
			expanded[expansionKey(node)] = true
			node = node.parent
		end
	end
end

-- Expand the selected logical node's ancestor chain when the selection changes
-- (e.g. clicked from the conflict list) — but only then, so a manual
-- collapse afterwards sticks.
local function revealSelected(view: any, projection: ExplorerProjection, explorerGeneration: number)
	local inst = world:get(cts.SelectedInstance, cts.SelectedInstance) :: Instance?
	local id = world:get(cts.SelectedExplorerId, cts.SelectedExplorerId) :: number?
	local revealRequest = (world:get(cts.ExplorerRevealGeneration, cts.ExplorerRevealGeneration) :: number?) or 0
	local viewChanged = view ~= lastRevealedView
	local explicitReveal = revealRequest ~= lastRevealRequest
	if
		(inst == nil and id == nil)
		or (
			inst == lastRevealedInstance
			and id == lastRevealedId
			and view == lastRevealedView
			and explorerGeneration == lastRevealedGeneration
			and revealRequest == lastRevealRequest
		)
	then
		return
	end
	lastRevealedInstance = inst
	lastRevealedId = id
	lastRevealedView = view
	lastRevealedGeneration = explorerGeneration
	lastRevealRequest = revealRequest
	expansionGen += 1
	local node = selectedNode(projection)
	if not node then
		return
	end
	if explicitReveal and searchText ~= "" then
		searchText = ""
		searchGen += 1
	end
	-- Center explicit conflict-list selections and carry that selection across
	-- version tabs. Clicking an already-visible Explorer row should not make
	-- the list jump underneath the pointer.
	if explicitReveal or viewChanged then
		pendingCenterNode = node
		pendingCenterView = view
	end
	node = node.parent
	while node do
		expanded[expansionKey(node)] = true
		node = node.parent
	end
end

local function explorerRow(node: ExplorerNode, depth: number, hasChildren: boolean, isExpanded: boolean): Row
	local classSuffix = if node.className then `  <font color="{theme.textDimHex}">{node.className}</font>` else ""
	return {
		node = node,
		source = node.source,
		subject = node.subject,
		expansionKey = expansionKey(node),
		label = `{format.escapeRich(node.name)}{classSuffix}`,
		searchText = string.lower(`{node.name} {node.className or ""}`),
		className = node.className,
		phantom = node.phantom,
		depth = depth,
		hasChildren = hasChildren,
		isExpanded = isExpanded,
	}
end

local function projectionRows(projection: ExplorerProjection, forceExpanded: boolean): { Row }
	local rows: { Row } = {}
	local function visit(node: ExplorerNode, depth: number)
		local isExpanded = forceExpanded or expanded[expansionKey(node)] == true
		table.insert(rows, explorerRow(node, depth, #node.children > 0, isExpanded))
		if isExpanded then
			for _, child in node.children do
				visit(child, depth + 1)
			end
		end
	end
	for _, root in projection.roots do
		visit(root, 0)
	end
	return rows
end

local TABS = { "base", "ours", "theirs", "result" }

local function tabColor(tab: string): Color3
	if tab == "ours" then
		return theme.ours
	elseif tab == "theirs" then
		return theme.theirs
	elseif tab == "base" then
		return theme.textMuted
	end
	return theme.text
end

local function TabBar()
	local viewTab = world:get(cts.ViewTab, cts.ViewTab)
	Element("Frame", {
		id = "tabs",
		Size = UDim2.new(1, 0, 0, 26),
		tags = { "bg-none", "flex-row", "gap-xs" },
	}, function()
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 4),
			PaddingRight = UDim.new(0, 4),
			PaddingTop = UDim.new(0, 3),
			PaddingBottom = UDim.new(0, 3),
		})
		for _, tab in TABS do
			local isActive = viewTab == tab
			Element("TextButton", {
				id = tab,
				Text = tab:upper(),
				Size = UDim2.new(0.25, -8, 1, 0),
				tags = { "corner-sm", "vs-border", "button" },
				BackgroundColor3 = if isActive then theme.selection else theme.surface,
				TextColor3 = if isActive then theme.textBright else tabColor(tab),
				TextSize = 11,
				events = {
					Activated = function()
						world:set(cts.ViewTab, cts.ViewTab, tab)
					end,
				},
			})
		end
	end)
end

-- Ancestors of the selected occurrence light up as on-path. Following the
-- projected parent (rather than Instance.Parent) gives reparent views and virtual
-- nodes their own topology.
local function ancestorSet(projection: ExplorerProjection): { [ExplorerNode]: boolean }
	local set: { [any]: boolean } = {}
	local node = selectedNode(projection)
	node = node and node.parent
	while node do
		set[node] = true
		node = node.parent
	end
	return set
end

local ROW_HEIGHT = 22
local HEADER_HEIGHT = 24
local TABS_HEIGHT = 26
local SEARCH_HEIGHT = 30

local rowsCacheKey: string? = nil
local rowsCache: { Row } = {}

-- Rows render through VirtualList: only the visible index range builds
-- TreeRows, so even a large search result stays cheap.
local function renderRows(rows: { Row }, searching: boolean, projection: ExplorerProjection, view: any)
	local onPath = ancestorSet(projection)
	local selected = selectedNode(projection)
	VirtualList.push({
		id = "rows",
		data = rows,
		dimensions = Dimensions.consistentSize(ROW_HEIGHT),
		direction = "y",
		Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT - TABS_HEIGHT - SEARCH_HEIGHT),
		tags = { "bg-none" },
	}, function(row: Row, index: number)
		TreeRow({
			id = "row",
			depth = row.depth,
			label = row.label,
			className = row.className,
			phantom = row.phantom,
			hasChildren = row.hasChildren,
			expanded = row.isExpanded,
			selected = row.node == selected,
			onPath = onPath[row.node] == true,
			stripe = index % 2 == 0,
			onToggle = if row.hasChildren and not searching
				then function()
					setExpanded(row.expansionKey, not row.isExpanded)
				end
				else nil,
			onActivated = if row.source
					or row.subject
					or row.node.id
				then function()
					local instance = row.source or row.subject
					if instance then
						world:set(cts.SelectedInstance, cts.SelectedInstance, instance)
					else
						world:remove(cts.SelectedInstance, cts.SelectedInstance)
					end
					if row.node.id then
						world:set(cts.SelectedExplorerId, cts.SelectedExplorerId, row.node.id)
					else
						world:remove(cts.SelectedExplorerId, cts.SelectedExplorerId)
					end
				end
				else nil,
		})
	end)

	-- VirtualList keeps its overlay scope open until pop(). Its retained parent
	-- owns the real ScrollingFrame, so an explicit Select can center the row
	-- after ancestor expansion has rebuilt this frame's flattened row array.
	if pendingCenterView == view and pendingCenterNode then
		local centerIndex: number? = nil
		for index, row in rows do
			if row.node == pendingCenterNode then
				centerIndex = index
				break
			end
		end
		local overlayNode = packages.miu.peek()
		local outerNode = overlayNode.parent
		local scrollNode = outerNode and outerNode.children.scroll
		local scroll = if scrollNode then scrollNode.instance :: ScrollingFrame else nil
		if centerIndex and scroll and scroll.AbsoluteWindowSize.Y > 0 then
			local viewportHeight = scroll.AbsoluteWindowSize.Y
			local canvasHeight = #rows * ROW_HEIGHT
			local target = (centerIndex - 0.5) * ROW_HEIGHT - viewportHeight / 2
			target = math.clamp(target, 0, math.max(0, canvasHeight - viewportHeight))
			scroll.CanvasPosition = Vector2.new(scroll.CanvasPosition.X, target)
			pendingCenterNode = nil
			pendingCenterView = nil
		end
	end
	VirtualList.pop()
end

local function normalizedSearch(): string
	return string.lower((searchText:match("^%s*(.-)%s*$") :: string?) or "")
end

local function rowMatches(row: Row, query: string): boolean
	for token in query:gmatch("%S+") do
		if not row.searchText:find(token, 1, true) then
			return false
		end
	end
	return true
end

-- Rows are pre-order, so each match's nearest preceding shallower rows are
-- precisely its projected visual ancestors even when Instance.Parent differs.
local function filterRows(rows: { Row }, query: string): { Row }
	if query == "" then
		return rows
	end

	local keep: { [number]: boolean } = {}
	for index, row in rows do
		if not rowMatches(row, query) then
			continue
		end
		keep[index] = true
		local ancestorDepth = row.depth
		for candidate = index - 1, 1, -1 do
			local candidateRow = rows[candidate]
			if candidateRow.depth < ancestorDepth then
				keep[candidate] = true
				ancestorDepth = candidateRow.depth
				if ancestorDepth == 0 then
					break
				end
			end
		end
	end

	local filtered: { Row } = {}
	for index, row in rows do
		if keep[index] then
			table.insert(filtered, row)
		end
	end
	return filtered
end

local function SearchBar()
	Element("Frame", {
		id = "search-wrap",
		Size = UDim2.new(1, 0, 0, SEARCH_HEIGHT),
		tags = { "bg-none" },
	}, function()
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 4),
			PaddingRight = UDim.new(0, 4),
			PaddingTop = UDim.new(0, 3),
			PaddingBottom = UDim.new(0, 3),
		})

		local textBox: TextBox
		local node = Element("TextBox", {
			id = "search",
			Text = searchText,
			PlaceholderText = "Search name or class…",
			ClearTextOnFocus = false,
			Size = UDim2.fromScale(1, 1),
			tags = { "text", "surface-high", "vs-border", "corner-sm" },
			TextColor3 = theme.text,
			PlaceholderColor3 = theme.textMuted,
			TextSize = theme.textSizeSmall,
			events = {
				Changed = function(property: string)
					if property ~= "Text" or textBox.Text == searchText then
						return
					end
					searchText = textBox.Text
					searchGen += 1
				end,
			},
		})
		textBox = node.instance :: TextBox
	end)
end

local function ExplorerPanel()
	Element("Frame", {
		id = "explorer",
		Size = UDim2.fromScale(1, 1),
		tags = { "bg-none", "flex-col" },
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = "MERGE EXPLORER",
			Size = UDim2.new(1, 0, 0, HEADER_HEIGHT),
			tags = { "header-band" },
		})

		if world:has(cts.Finished, cts.Finished) then
			return
		end

		TabBar()
		SearchBar()

		local viewTab = world:get(cts.ViewTab, cts.ViewTab) :: string
		local view = VIEW_ENTITIES[viewTab]
		local explorerGen = (world:get(cts.ExplorerGeneration, cts.ExplorerGeneration) :: number?) or 0
		local projection = view and world:get(view, cts.ExplorerProjection) :: ExplorerProjection?
		-- Systems begin running as they are registered. The first UI frame can
		-- therefore precede the mirror/projection's initial Heartbeat; an empty
		-- panel is the correct intermediate state and avoids querying a nil view.
		if not view or not projection or explorerGen == 0 then
			return
		end
		seedExpansion(view, projection)
		revealSelected(view, projection, explorerGen)

		-- Rows are derived data: rebuilt only when the projected tree,
		-- expansion, or tab changes. Selection/on-path stay out of the key —
		-- renderRows applies them per frame over the cached array.
		local query = normalizedSearch()
		local searching = query ~= ""
		local key = `{viewTab}:{expansionGen}:{explorerGen}:{searchGen}`
		if key ~= rowsCacheKey then
			rowsCacheKey = key
			debug.profilebegin("gitrbx_explorer_rows")
			local rows = projectionRows(projection, searching)
			rowsCache = filterRows(rows, query)
			debug.profileend()
		end
		renderRows(rowsCache, searching, projection, view)
	end)
end

return ExplorerPanel
