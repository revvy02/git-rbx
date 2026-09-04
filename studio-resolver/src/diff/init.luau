--[[
	Diff viewer mode: `git rbx diff old new --studio`.

	Rust stamped the diff document into a temporary copy of the new file
	(a __GitRbxDiff container; see src/diff_container.rs). This mode reads
	it and shows the new version with every changed instance highlighted by
	kind, removed subtrees restored as translucent ghosts at their old
	location, and one "changes" dock widget: the PatchVisualizer over the
	whole document. Double click a row to select the live instance and frame
	it; Studio's own Explorer and Properties do the rest.

	Read-only: nothing here writes to disk, and the session ends when the
	changes widget is closed.
]]

local CollectionService = game:GetService("CollectionService")
local HttpService = game:GetService("HttpService")
local RunService = game:GetService("RunService")
local Selection = game:GetService("Selection")

local packages = require("./packages")
local Element = packages.miu.Element
local Portal = packages.miu.Portal

local PatchVisualizer = require("./ui/PatchVisualizer")
local fromDocument = require("./ui/PatchVisualizer/fromDocument")
local format = require("./library/format")
local theme = require("./library/theme")
local widgets = require("./ecs/systems/library/widgets")
local getBoundingBox = require("./library/viewport/getBoundingBox")
local getIdealCameraCFrame = require("./library/viewport/getIdealCameraCFrame")

local CONTAINER_NAME = "__GitRbxDiff"
local GHOST_TAG = "GitRbxGhost"
local HIGHLIGHT_TAG = "GitRbxDiffHighlight"

type ManifestNode = fromDocument.ManifestNode

local function readChunks(folder: Instance?): string
	if not folder then
		return ""
	end
	local chunks = folder:GetChildren()
	table.sort(chunks, function(a, b)
		return a.Name < b.Name
	end)
	local parts = {}
	for _, chunk in chunks do
		if chunk:IsA("StringValue") then
			table.insert(parts, chunk.Value)
		end
	end
	return table.concat(parts)
end

local function readManifest(folder: Instance?): { ManifestNode }
	local encoded = readChunks(folder)
	if encoded == "" then
		return {}
	end
	local nodes = {}
	for index, record in HttpService:JSONDecode(encoded) do
		nodes[index] = {
			id = record[1],
			parent = if record[2] == 0 then nil else record[2],
			name = record[3],
			className = record[4],
		}
	end
	return nodes
end

local function colorOf(kind: string): Color3
	if kind == "Add" then
		return theme.added
	elseif kind == "Remove" then
		return theme.removed
	elseif kind == "Reparent" then
		return theme.reparented
	elseif kind == "Pivot" then
		return theme.pivoted
	end
	return theme.modified
end

local function highlight(target: Instance, kind: string, fill: number)
	if not (target:IsA("PVInstance") and target:IsDescendantOf(workspace)) then
		return
	end
	local color = colorOf(kind)
	local h = Instance.new("Highlight")
	h.Name = "GitRbxDiffHighlight"
	h.Adornee = target
	h.OutlineColor = color
	h.FillColor = color
	h.FillTransparency = fill
	h.Parent = target
	CollectionService:AddTag(h, HIGHLIGHT_TAG)
end

-- Removed subtrees come back as ghosts: parented where they used to be when
-- that parent still exists, translucent, non-colliding, outlined in the
-- remove color.
local function restoreGhost(snapshotFolder: Instance, subjects: { [number]: Instance })
	local root = snapshotFolder:GetChildren()[1]
	if not root then
		return nil
	end
	local oldParent = snapshotFolder:GetAttribute("OldParent")
	local parent = if typeof(oldParent) == "number" then subjects[oldParent] else nil
	root.Parent = parent or workspace
	CollectionService:AddTag(root, GHOST_TAG)
	for _, descendant in root:GetDescendants() do
		if descendant:IsA("BasePart") then
			descendant.Transparency = 0.6 + 0.4 * descendant.Transparency
			descendant.CanCollide = false
			descendant.CanQuery = false
			descendant.Anchored = true
		end
	end
	if root:IsA("BasePart") then
		root.Transparency = 0.6 + 0.4 * root.Transparency
		root.CanCollide = false
		root.Anchored = true
	end
	highlight(root, "Remove", 0.85)
	return root
end

local function focus(target: Instance)
	if not target:IsA("PVInstance") then
		return
	end
	local camera = workspace.CurrentCamera
	local cframe, size = getBoundingBox(target)
	camera.CFrame = getIdealCameraCFrame(cframe, size, {
		fov = camera.FieldOfView,
		aspectRatio = camera.ViewportSize.X / camera.ViewportSize.Y,
	})
	camera.Focus = cframe
end

return function(args: { file: string }): string
	local container = game:FindFirstChild(CONTAINER_NAME, true)
	if not container then
		warn(`no {CONTAINER_NAME} container in this place — nothing to show`)
		return "no-diff"
	end

	local treesFolder = container:FindFirstChild("VirtualTrees")
	local old = readManifest(treesFolder and treesFolder:FindFirstChild("Old"))
	local new = readManifest(treesFolder and treesFolder:FindFirstChild("New"))

	local subjects: { [number]: Instance } = {}
	local subjectsFolder = treesFolder and treesFolder:FindFirstChild("Subjects")
	if subjectsFolder then
		for _, value in subjectsFolder:GetChildren() do
			if value:IsA("ObjectValue") and value.Value then
				local id = tonumber(string.sub(value.Name, 2))
				if id then
					subjects[id] = value.Value
				end
			end
		end
	end

	local body = HttpService:JSONDecode(readChunks(container:FindFirstChild("Document")))
	local ops = body.ops or {}
	local pivotRows = {}
	for _, pivot in body.pivots or {} do
		local components = pivot.delta
		local delta = CFrame.new(table.unpack(components))
		table.insert(pivotRows, {
			id = pivot.id,
			current = "old placement",
			incoming = format.escapeRich(format.formatDelta(delta)),
		})
	end

	-- Ghosts for removed subtrees, and one highlight per changed root.
	local removedFolder = container:FindFirstChild("Removed")
	local ghosts: { [number]: Instance } = {}
	if removedFolder then
		for _, snapshot in removedFolder:GetChildren() do
			local id = snapshot:GetAttribute("Id")
			local ghost = restoreGhost(snapshot, subjects)
			if ghost and typeof(id) == "number" then
				ghosts[id] = ghost
				subjects[id] = ghost
			end
		end
	end
	for _, op in ops do
		local kind = if op.op == "add"
			then "Add"
			elseif op.op == "reparent" then "Reparent"
			elseif op.op == "remove" then nil
			else "Edit"
		local target = subjects[op.id]
		if kind and target then
			highlight(target, kind, if kind == "Edit" then 1 else 0.9)
		end
	end
	for _, pivot in body.pivots or {} do
		local target = subjects[pivot.id]
		if target then
			highlight(target, "Pivot", 0.9)
		end
	end

	local patch = fromDocument({
		ops = ops,
		old = old,
		new = new,
		pivots = pivotRows,
		addProperties = true,
	})

	local counts = {
		added = container:GetAttribute("Added") or 0,
		removed = container:GetAttribute("Removed") or 0,
		modified = container:GetAttribute("Modified") or 0,
		reparented = container:GetAttribute("Reparented") or 0,
		pivoted = container:GetAttribute("Pivoted") or 0,
	}
	local title = string.format(
		"%d ADDED · %d REMOVED · %d MODIFIED · %d REPARENTED · %d PIVOTED",
		counts.added,
		counts.removed,
		counts.modified,
		counts.reparented,
		counts.pivoted
	)

	local widget = widgets.create("GitRbxDiffChanges", "git-rbx — changes", Enum.InitialDockState.Right, {
		width = 520,
		height = 640,
		minWidth = 360,
		minHeight = 240,
	})

	local selectedId: number? = nil
	local function select(node)
		local id = node.id :: number
		local target = subjects[id]
		if not target then
			return
		end
		selectedId = id
		Selection:Set({ target })
		focus(target)
	end

	print(`git-rbx diff viewer: {#ops} op(s), {#(body.pivots or {})} pivot(s) — {args.file}`)

	local connection = RunService.Heartbeat:Connect(function()
		debug.profilebegin("gitrbx_diff_changes")
		Portal(widget, "git-rbx-diff-changes", function()
			theme.link()
			Element("Frame", {
				id = "changes",
				Size = UDim2.fromScale(1, 1),
				tags = { "surface", "vs-border" },
			}, function()
				Element("TextLabel", {
					id = "title",
					Text = title,
					Size = UDim2.new(1, 0, 0, 24),
					tags = { "header-band" },
				})
				PatchVisualizer({
					patch = patch,
					stateKey = container,
					headers = { "Property", "Old", "New" },
					selectedId = selectedId,
					onSelect = select,
					emptyText = "No semantic differences",
					Size = UDim2.new(1, 0, 1, -24),
					Position = UDim2.fromOffset(0, 24),
				})
			end)
		end)
		debug.profileend()
	end)

	-- The session lives until the changes widget is closed.
	while widget.Enabled do
		task.wait(0.25)
	end
	connection:Disconnect()
	return `closed — {args.file}`
end
