--[[
	rbx-diff studio resolver — visual front end over the in-file conflict state.

	Run against a Studio that has the conflicted place open:

	  rodeo run --place ./merged.rbxl --focus --detach src -- ./merged.rbxl

	Discovery is entirely in-DOM (the __RbxDiffMerge container + RbxDiffConflict
	tags that `rbx-diff merge` stamped). Decisions shell out host-side via
	@rodeo/process to the canonical CLI:

	  rbx-diff resolve <file> --take <side> --entry <Conflict_N>
	  rbx-diff resolve <file> --finalize

	The Studio DOM is a COPY of the file — the disk file stays authoritative and
	only rbx-diff writes it. This script is just eyes and buttons.

	Args: <merged-file> [--rbx-diff <path-to-binary>]
]]

local RunService = game:GetService("RunService")
local CoreGui = game:GetService("CoreGui")

local process = require("@rodeo/process")

local argparse = require("@pkg/argparse")

-- Leaf requires: rodeo's darklua bundling can't resolve a require to an
-- init.luau, so miu's aggregator (src/miu/init.luau) can't be the target —
-- the vendored package keeps its upstream shape regardless.
local Element = require("@self/miu/Instance")
local Portal = require("@self/miu/Portal")

local CONTAINER_NAME = "__RbxDiffMerge"

local UNRESOLVED_COLOR = Color3.fromRGB(255, 170, 0)
local OURS_COLOR = Color3.fromRGB(70, 160, 255)
local THEIRS_COLOR = Color3.fromRGB(170, 90, 255)

local parser = argparse("rbx-diff-resolver", "Resolve rbx-diff merge conflicts visually in Studio")
parser:argument("file", "Conflicted merged file written by `rbx-diff merge`")
parser:option("--rbx-diff", "Path to the rbx-diff binary"):target("rbxDiffBin"):default("rbx-diff")

-- pparse: argparse's error path calls process.exit, which suspends the run
-- under rodeo's lune adapter — fail with a readable return instead
local parseOk, args = parser:pparse(process.args)
if not parseOk then
	warn(`{args}`)
	return parser:get_usage()
end

local mergedFile = args.file
local rbxDiffBin = args.rbxDiffBin

-- ============================================================================
-- Conflict model: read once from the DOM. Resolution state is kept locally
-- and mirrored onto the live entries so other in-Studio tools see it too.
-- ============================================================================

type Conflict = {
	name: string,
	kind: string,
	path: string,
	property: string?,
	resolved: string?,
	target: Instance?,
	entry: Instance,
	pending: boolean,
}

local container = game:FindFirstChild(CONTAINER_NAME, true)
if not container then
	warn(`no {CONTAINER_NAME} container in this place — nothing to resolve`)
	return "no-conflicts"
end

local conflicts: { Conflict } = {}
for _, entry in container:GetChildren() do
	local kind = entry:GetAttribute("Kind")
	if typeof(kind) ~= "string" then
		continue
	end
	local targetValue = entry:FindFirstChild("Target")
	local resolved = entry:GetAttribute("Resolved")
	table.insert(conflicts, {
		name = entry.Name,
		kind = kind,
		path = entry:GetAttribute("Path") or "?",
		property = entry:GetAttribute("Property"),
		resolved = if typeof(resolved) == "string" and resolved ~= "" then resolved else nil,
		target = if targetValue and targetValue:IsA("ObjectValue") then targetValue.Value else nil,
		entry = entry,
		pending = false,
	})
end
table.sort(conflicts, function(a, b)
	return a.name < b.name
end)

print(`rbx-diff resolver: {#conflicts} conflict(s) in {mergedFile}`)

local function statusColor(conflict: Conflict): Color3
	if conflict.resolved == "ours" then
		return OURS_COLOR
	elseif conflict.resolved == "theirs" then
		return THEIRS_COLOR
	end
	return UNRESOLVED_COLOR
end

-- ============================================================================
-- Decision transport: the canonical CLI, host-side
-- ============================================================================

local finished = false
local finalizing = false
local finalizeError: string? = nil

local function emit(conflict: Conflict, side: string)
	if conflict.pending or conflict.resolved == side then
		return
	end
	conflict.pending = true
	task.spawn(function()
		local result = process.run({ rbxDiffBin, "resolve", mergedFile, "--take", side, "--entry", conflict.name })
		if result.ok then
			conflict.resolved = side
			conflict.entry:SetAttribute("Resolved", side)
		else
			warn(`rbx-diff --take failed for {conflict.name}: {result.stderr}`)
		end
		conflict.pending = false
	end)
end

local function finalize()
	if finalizing then
		return
	end
	finalizing = true
	task.spawn(function()
		local result = process.run({ rbxDiffBin, "resolve", mergedFile, "--finalize" })
		if result.ok then
			finished = true
		else
			finalizeError = result.stderr
			warn(`finalize failed: {result.stderr}`)
		end
		finalizing = false
	end)
end

local function focusOn(conflict: Conflict)
	local target = conflict.target
	if not target or not target:IsA("PVInstance") then
		return
	end
	local ok, pivot = pcall(target.GetPivot, target)
	if ok then
		local camera = workspace.CurrentCamera
		camera.CFrame = CFrame.lookAt(pivot.Position + Vector3.new(12, 8, 12), pivot.Position)
	end
end

-- ============================================================================
-- 3D highlights: plain instances keyed by conflict, colored by state
-- (miu owns only the 2D panel)
-- ============================================================================

local liveHighlights: { [string]: Highlight } = {}

local function updateHighlights()
	for _, conflict in conflicts do
		local target = conflict.target
		local spatial = target ~= nil
			and target:IsDescendantOf(workspace)
			and target:IsA("PVInstance")
		if not spatial or finished then
			local existing = liveHighlights[conflict.name]
			if existing then
				existing:Destroy()
				liveHighlights[conflict.name] = nil
			end
			continue
		end

		local highlight = liveHighlights[conflict.name]
		if not highlight then
			highlight = Instance.new("Highlight")
			highlight.Name = `RbxDiffHighlight_{conflict.name}`
			highlight.Adornee = target
			highlight.Parent = target
			liveHighlights[conflict.name] = highlight
		end
		local color = statusColor(conflict)
		highlight.OutlineColor = color
		highlight.FillColor = color
		highlight.FillTransparency = 0.85
	end
end

-- ============================================================================
-- UI (immediate mode, CoreGui overlay — the same surface rojo mounts on)
-- ============================================================================

local function Label(id: string, text: string, size: UDim2, color: Color3?)
	Element("TextLabel", {
		id = id,
		Text = text,
		Size = size,
		BackgroundTransparency = 1,
		TextColor3 = color or Color3.fromRGB(230, 230, 230),
		TextXAlignment = Enum.TextXAlignment.Left,
		TextSize = 14,
		Font = Enum.Font.Gotham,
		TextTruncate = Enum.TextTruncate.AtEnd,
	})
end

local function Button(id: string, text: string, highlighted: boolean, color: Color3, onClick: () -> ())
	Element("TextButton", {
		id = id,
		Text = text,
		Size = UDim2.new(0, 64, 0, 22),
		BackgroundColor3 = if highlighted then color else Color3.fromRGB(60, 60, 65),
		TextColor3 = Color3.fromRGB(240, 240, 240),
		TextSize = 13,
		Font = Enum.Font.GothamMedium,
		AutoButtonColor = true,
		events = { Activated = onClick },
	}, function()
		Element("UICorner", { id = "corner", CornerRadius = UDim.new(0, 4) })
	end)
end

local function ConflictRow(conflict: Conflict)
	Element("Frame", {
		id = conflict.name,
		Size = UDim2.new(1, 0, 0, 62),
		BackgroundColor3 = Color3.fromRGB(40, 40, 46),
	}, function()
		Element("UICorner", { id = "corner", CornerRadius = UDim.new(0, 6) })
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 8),
			PaddingRight = UDim.new(0, 8),
			PaddingTop = UDim.new(0, 4),
			PaddingBottom = UDim.new(0, 4),
		})

		Label("path", conflict.path, UDim2.new(1, 0, 0, 16), statusColor(conflict))

		local detail = conflict.kind
			.. (if conflict.property then ` ({conflict.property})` else "")
			.. (if conflict.pending then "  …" else "")
		Element("TextLabel", {
			id = "detail",
			Text = detail,
			Size = UDim2.new(1, 0, 0, 12),
			Position = UDim2.new(0, 8, 0, 20),
			BackgroundTransparency = 1,
			TextColor3 = Color3.fromRGB(160, 160, 170),
			TextXAlignment = Enum.TextXAlignment.Left,
			TextSize = 12,
			Font = Enum.Font.Gotham,
		})

		Element("Frame", {
			id = "buttons",
			Size = UDim2.new(1, -16, 0, 22),
			Position = UDim2.new(0, 8, 1, -26),
			BackgroundTransparency = 1,
		}, function()
			Element("UIListLayout", {
				id = "layout",
				FillDirection = Enum.FillDirection.Horizontal,
				Padding = UDim.new(0, 6),
			})
			Button("ours", "Ours", conflict.resolved == "ours", OURS_COLOR, function()
				emit(conflict, "ours")
			end)
			Button("theirs", "Theirs", conflict.resolved == "theirs", THEIRS_COLOR, function()
				emit(conflict, "theirs")
			end)
			if conflict.target and conflict.target:IsA("PVInstance") and conflict.target:IsDescendantOf(workspace) then
				Button("view", "View", false, Color3.fromRGB(80, 80, 90), function()
					focusOn(conflict)
				end)
			end
		end)
	end)
end

local function Panel()
	local unresolved = 0
	for _, conflict in conflicts do
		if not conflict.resolved then
			unresolved += 1
		end
	end

	Element("Frame", {
		id = "panel",
		Size = UDim2.new(0, 340, 0, math.min(96 + #conflicts * 68, 620)),
		Position = UDim2.new(1, -356, 0, 16),
		BackgroundColor3 = Color3.fromRGB(28, 28, 32),
	}, function()
		Element("UICorner", { id = "corner", CornerRadius = UDim.new(0, 8) })
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 10),
			PaddingRight = UDim.new(0, 10),
			PaddingTop = UDim.new(0, 10),
			PaddingBottom = UDim.new(0, 10),
		})
		Element("UIListLayout", {
			id = "layout",
			Padding = UDim.new(0, 6),
			SortOrder = Enum.SortOrder.LayoutOrder,
		})

		Label(
			"title",
			if finished
				then "Merge finalized — file written"
				else `rbx-diff: {unresolved}/{#conflicts} unresolved`,
			UDim2.new(1, 0, 0, 20),
			if finished then Color3.fromRGB(120, 220, 140) else nil
		)

		if finished then
			return
		end

		for _, conflict in conflicts do
			ConflictRow(conflict)
		end

		local canFinalize = unresolved == 0 and not finalizing
		Element("TextButton", {
			id = "finalize",
			Text = if finalizing
				then "Finalizing…"
				elseif finalizeError then "Finalize failed — retry"
				else "Finalize",
			Size = UDim2.new(1, 0, 0, 30),
			BackgroundColor3 = if canFinalize
				then Color3.fromRGB(60, 160, 90)
				else Color3.fromRGB(55, 55, 60),
			TextColor3 = Color3.fromRGB(240, 240, 240),
			TextSize = 14,
			Font = Enum.Font.GothamMedium,
			events = {
				Activated = function()
					if canFinalize then
						finalize()
					end
				end,
			},
		}, function()
			Element("UICorner", { id = "corner", CornerRadius = UDim.new(0, 6) })
		end)
	end)
end

-- ============================================================================
-- Frame loop; the run stays alive until finalized
-- ============================================================================

local connection = RunService.Heartbeat:Connect(function()
	updateHighlights()
	Portal(CoreGui, "rbx-diff-resolver", function()
		Element("ScreenGui", {
			id = "gui",
			Name = "RbxDiffResolver",
			ResetOnSpawn = false,
			DisplayOrder = 200,
			ZIndexBehavior = Enum.ZIndexBehavior.Sibling,
		}, function()
			Panel()
		end)
	end)
end)

while not finished do
	task.wait(0.25)
end
task.wait(2) -- let the success state render briefly

connection:Disconnect()
for _, highlight in liveHighlights do
	highlight:Destroy()
end

return `finalized {#conflicts} conflict(s) — {mergedFile} is clean`
