--[[
	Adapter from the diff document's op vocabulary (src/diff_document.rs) to
	the visualizer's Patch. Used by the conflict resolver (one side's impact
	against base/side manifests) and by the diff viewer (the whole document
	against old/new manifests).

	Nodes are materialized from the manifests: an op's instance plus the
	ancestor chain that reaches it. Ops that create instances (add) live in
	the new manifest; everything else addresses the old one, and a reparented
	instance is shown under its new parent, as Rojo places an added node
	under its destination.
]]

local format = require("../../library/format")
local types = require("./types")

export type ManifestNode = { id: number, parent: number?, name: string, className: string }
export type Manifest = { ManifestNode }

export type Options = {
	ops: { any }, -- cts.DocumentOp list
	old: Manifest,
	new: Manifest,
	-- Optional pivot rows: the boundary's id plus RichText for both columns.
	pivots: { { id: number, current: string, incoming: string } }?,
	-- Whether add rows list the added instance's properties (Rojo does).
	addProperties: boolean?,
}

type Index = { byId: { [number]: ManifestNode } }

local function index(manifest: Manifest): Index
	local byId = {}
	for _, node in manifest do
		byId[node.id] = node
	end
	return { byId = byId }
end

local function pathOf(idx: Index, id: number?): string
	local parts = {}
	local cursor = if id then idx.byId[id] else nil
	while cursor do
		table.insert(parts, 1, cursor.name)
		cursor = if cursor.parent then idx.byId[cursor.parent] else nil
	end
	return table.concat(parts, ".")
end

local function fromDocument(options: Options): types.Patch
	local oldIdx = index(options.old)
	local newIdx = index(options.new)
	local nodes: { [types.Id]: types.PatchNodeInput } = {}

	-- Materialize a node from a manifest, with its ancestors, preferring the
	-- requested version and falling back to the other for shared ancestors.
	local function ensure(id: number, preferNew: boolean): types.PatchNodeInput?
		local existing = nodes[id]
		if existing then
			return existing
		end
		local first = if preferNew then newIdx else oldIdx
		local second = if preferNew then oldIdx else newIdx
		local record = first.byId[id] or second.byId[id]
		if not record then
			return nil
		end
		local node: types.PatchNodeInput = {
			id = id,
			name = record.name,
			className = record.className,
			parent = record.parent,
			rows = {},
		}
		nodes[id] = node
		if record.parent then
			ensure(record.parent, preferNew)
		end
		return node
	end

	local function rowsOf(node: types.PatchNodeInput): { types.ChangeRow }
		return node.rows :: { types.ChangeRow }
	end

	for _, op in options.ops do
		if op.op == "add" then
			local root = ensure(op.id, true)
			if not root then
				continue
			end
			root.kind = "Add"
			root.subtreeCount = op.instanceCount
			for _, added in op.subtree or {} do
				local node = nodes[added.id]
				if not node then
					node = {
						id = added.id,
						name = added.name,
						className = added.class,
						parent = added.parent,
						rows = {},
					}
					nodes[added.id] = node
					if added.parent then
						ensure(added.parent, true)
					end
				end
				node.kind = "Add"
				if options.addProperties then
					for property, value in added.properties or {} do
						table.insert(rowsOf(node), { property = property, kind = "Add", incoming = value })
					end
				end
			end
		elseif op.op == "remove" then
			local node = ensure(op.id, false)
			if node then
				node.kind = "Remove"
				node.subtreeCount = op.instanceCount
			end
		elseif op.op == "reparent" then
			-- Shown under the destination: the node's parent is the new one.
			local node = ensure(op.id, true)
			if node then
				node.parent = op.to
				if op.to then
					ensure(op.to, true)
				end
				table.insert(rowsOf(node), {
					property = "Parent",
					kind = "Reparent",
					currentText = format.escapeRich(pathOf(oldIdx, op.from)),
					incomingText = format.escapeRich(pathOf(newIdx, op.to)),
				})
			end
		elseif op.op == "setName" then
			local node = ensure(op.id, true)
			if node then
				table.insert(rowsOf(node), {
					property = "Name",
					kind = "Edit",
					currentText = `"{format.escapeRich(tostring(op.before))}"`,
					incomingText = `"{format.escapeRich(tostring(op.after))}"`,
				})
			end
		elseif op.op == "setProperty" then
			local node = ensure(op.id, true)
			if node then
				table.insert(rowsOf(node), {
					property = op.property or "Property",
					kind = "Edit",
					current = op.before,
					incoming = op.after,
				})
			end
		end
	end

	for _, pivot in options.pivots or {} do
		local node = ensure(pivot.id, true)
		if node then
			node.kind = node.kind or "Pivot"
			table.insert(rowsOf(node), {
				property = "Pivot",
				kind = "Pivot",
				currentText = pivot.current,
				incomingText = pivot.incoming,
			})
		end
	end

	return { nodes = nodes }
end

return fromDocument
