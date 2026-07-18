--[[
	rbx-diff studio resolver — visual front end over the in-file conflict state.

	Run against a Studio that has the conflicted place open:

	  rodeo run --place ./merged.rbxl --focus --detach src -- ./merged.rbxl

	Discovery is entirely in-DOM (the __RbxDiffMerge container + RbxDiffConflict
	tags that `rbx-diff merge` stamped). Decisions are STAGED in the session —
	click Ours/Theirs to pick (click again to unpick), change freely — and
	nothing touches the disk file until Complete, which batches the canonical
	CLI calls host-side via @rodeo/process:

	  rbx-diff resolve <file> --take <side> --entry <Conflict_N>   (per decision)
	  rbx-diff resolve <file> --finalize

	The Studio DOM is a COPY of the file — the disk file stays authoritative and
	only rbx-diff writes it, only at Complete. Abandoning the session leaves the
	conflicted file exactly as `rbx-diff merge` wrote it.

	Args: <merged-file> [--rbx-diff <path-to-binary>]
]]

local RunService = game:GetService("RunService")
local CoreGui = game:GetService("CoreGui")
local HttpService = game:GetService("HttpService")

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
local CUSTOM_COLOR = Color3.fromRGB(80, 200, 160)

local parser = argparse("rbx-diff-resolver", "Resolve rbx-diff merge conflicts visually in Studio")
parser:argument("file", "Conflicted merged file written by `rbx-diff merge`")
parser:option("--rbx-diff", "Path to the rbx-diff binary"):target("rbxDiffBin"):default("rbx-diff")
parser:option("--auto", "Debug: stage every conflict to this side and Complete immediately"):target("autoSide")

-- pparse: argparse's error path calls process.exit, which suspends the run
-- under rodeo's lune adapter — fail with a readable return instead
local args = parser:parse(process.args)
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
}

local container = game:FindFirstChild(CONTAINER_NAME, true)

-- Model files can't be opened as a place: import the conflicted rbxm into a
-- preview folder in Workspace instead. The disk file stays authoritative
-- exactly as with places — the import is only ever eyes and highlights.
if not container and mergedFile:match("%.rbxmx?$") then
	local roblox = require("@rodeo/roblox")
	local stalePreview = workspace:FindFirstChild("RbxDiffModelPreview")
	if stalePreview then
		stalePreview:Destroy()
	end
	local preview = Instance.new("Folder")
	preview.Name = "RbxDiffModelPreview"
	for _, root in roblox.import(mergedFile) do
		root.Parent = preview
	end
	preview.Parent = workspace
	container = game:FindFirstChild(CONTAINER_NAME, true)
end

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
	})
end
table.sort(conflicts, function(a, b)
	return a.name < b.name
end)

-- A previous resolver run in this Studio leaves its ScreenGui behind
-- (portal targets outlive the run) — clear it so sessions don't stack
local stale = CoreGui:FindFirstChild("RbxDiffResolver")
if stale then
	stale:Destroy()
end

print(`rbx-diff resolver: {#conflicts} conflict(s) in {mergedFile}`)

local function statusColor(conflict: Conflict): Color3
	if conflict.resolved == "ours" then
		return OURS_COLOR
	elseif conflict.resolved == "theirs" then
		return THEIRS_COLOR
	elseif conflict.resolved == "custom" then
		return CUSTOM_COLOR
	end
	return UNRESOLVED_COLOR
end

-- The live value of the conflicted property on the (copy) target instance.
local function liveValue(conflict: Conflict): (boolean, any)
	local target = conflict.target
	if not target then
		return false, nil
	end
	if conflict.property == "Name" then
		return true, target.Name
	end
	return pcall(function()
		return (target :: any)[conflict.property :: string]
	end)
end

-- Encode a captured value as the plain JSON the CLI coerces by property type.
-- Returns nil for types custom resolution doesn't support.
local function encodeValue(v: any): any
	local t = typeof(v)
	if t == "number" or t == "string" or t == "boolean" then
		return v
	elseif t == "Color3" then
		return { v.R, v.G, v.B }
	elseif t == "Vector3" then
		return { v.X, v.Y, v.Z }
	elseif t == "Vector2" then
		return { v.X, v.Y }
	elseif t == "CFrame" then
		return { v:GetComponents() }
	elseif t == "UDim" then
		return { v.Scale, v.Offset }
	elseif t == "UDim2" then
		return { v.X.Scale, v.X.Offset, v.Y.Scale, v.Y.Offset }
	end
	return nil
end

-- Custom = "edit the instance in Studio, that edit is the resolution".
-- Available for Property conflicts whose live value has an encodable type.
local function canCustom(conflict: Conflict): boolean
	if conflict.kind ~= "Property" or not conflict.target then
		return false
	end
	local ok, v = liveValue(conflict)
	return ok and encodeValue(v) ~= nil
end

-- ============================================================================
-- Decision transport: the canonical CLI, host-side
-- ============================================================================

local finished = false
local completing = false
local completeError: string? = nil

-- Staged decision: session-local only, freely revisable. Clicking the chosen
-- side again unpicks it. Nothing is written anywhere until Complete.
local function choose(conflict: Conflict, side: string)
	if completing or finished then
		return
	end
	conflict.resolved = if conflict.resolved == side then nil else side
end

-- Commit every staged decision through the canonical CLI, then finalize.
local function complete()
	if completing or finished then
		return
	end
	completing = true
	completeError = nil
	task.spawn(function()
		for _, conflict in conflicts do
			local args
			if conflict.resolved == "custom" then
				-- Capture the live value NOW — the final state of the user's
				-- in-Studio edit is the resolution
				local ok, v = liveValue(conflict)
				local encoded = if ok then encodeValue(v) else nil
				if encoded == nil then
					completeError = `{conflict.name}: could not capture custom value`
					warn(completeError)
					completing = false
					return
				end
				args = {
					rbxDiffBin,
					"resolve",
					mergedFile,
					"--take",
					"custom",
					"--entry",
					conflict.name,
					"--value",
					HttpService:JSONEncode(encoded),
				}
			else
				args = {
					rbxDiffBin,
					"resolve",
					mergedFile,
					"--take",
					conflict.resolved :: string,
					"--entry",
					conflict.name,
				}
			end
			local result = process.run(args)
			if not result.ok then
				completeError = `--take {conflict.name} failed: {result.stderr}`
				warn(completeError)
				completing = false
				return
			end
		end
		local result = process.run({ rbxDiffBin, "resolve", mergedFile, "--finalize" })
		if result.ok then
			finished = true
		else
			completeError = `finalize failed: {result.stderr}`
			warn(completeError)
		end
		completing = false
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
		local spatial = target ~= nil and target:IsDescendantOf(workspace) and target:IsA("PVInstance")
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

local function Label(id: string, text: string, color: Color3?)
	Element("TextLabel", {
		id = id,
		Text = text,
		Size = UDim2.new(1, 0, 0, 0),
		AutomaticSize = Enum.AutomaticSize.Y,
		BackgroundTransparency = 1,
		TextColor3 = color or Color3.fromRGB(230, 230, 230),
		TextXAlignment = Enum.TextXAlignment.Left,
		TextSize = 14,
		Font = Enum.Font.Gotham,
		TextWrapped = true,
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
		Size = UDim2.new(1, 0, 0, 0),
		AutomaticSize = Enum.AutomaticSize.Y,
		BackgroundColor3 = Color3.fromRGB(40, 40, 46),
	}, function()
		Element("UICorner", { id = "corner", CornerRadius = UDim.new(0, 6) })
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 8),
			PaddingRight = UDim.new(0, 8),
			PaddingTop = UDim.new(0, 6),
			PaddingBottom = UDim.new(0, 6),
		})
		Element("UIListLayout", {
			id = "layout",
			Padding = UDim.new(0, 4),
			SortOrder = Enum.SortOrder.LayoutOrder,
		})

		Label("path", conflict.path, statusColor(conflict))

		local detail = conflict.kind .. (if conflict.property then ` ({conflict.property})` else "")
		Element("TextLabel", {
			id = "detail",
			Text = detail,
			Size = UDim2.new(1, 0, 0, 0),
			AutomaticSize = Enum.AutomaticSize.Y,
			BackgroundTransparency = 1,
			TextColor3 = Color3.fromRGB(160, 160, 170),
			TextXAlignment = Enum.TextXAlignment.Left,
			TextSize = 12,
			Font = Enum.Font.Gotham,
			TextWrapped = true,
		})

		Element("Frame", {
			id = "buttons",
			Size = UDim2.new(1, 0, 0, 22),
			BackgroundTransparency = 1,
		}, function()
			Element("UIListLayout", {
				id = "layout",
				FillDirection = Enum.FillDirection.Horizontal,
				Padding = UDim.new(0, 6),
			})
			Button("ours", "Ours", conflict.resolved == "ours", OURS_COLOR, function()
				choose(conflict, "ours")
			end)
			Button("theirs", "Theirs", conflict.resolved == "theirs", THEIRS_COLOR, function()
				choose(conflict, "theirs")
			end)
			if canCustom(conflict) then
				Button("custom", "Custom", conflict.resolved == "custom", CUSTOM_COLOR, function()
					choose(conflict, "custom")
				end)
			end
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

	-- Cap the scrolling row list so the panel's automatic height stops at
	-- the bottom of the viewport; past that the canvas extends and scrolls.
	local viewport = workspace.CurrentCamera and workspace.CurrentCamera.ViewportSize.Y or 1080
	local chromeHeight = 16 + 20 + 30 + (10 * 2) + (6 * 2) + 16 -- top margin, title, button, padding, gaps, bottom margin
	local maxListHeight = math.max(120, viewport - chromeHeight)

	Element("Frame", {
		id = "panel",
		Size = UDim2.new(0, 340, 0, 0),
		AutomaticSize = Enum.AutomaticSize.Y,
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
				then "Merge complete — file written"
				else `rbx-diff: {#conflicts - unresolved}/{#conflicts} decided`,
			if finished then Color3.fromRGB(120, 220, 140) else nil
		)

		if finished then
			return
		end

		Element("ScrollingFrame", {
			id = "rows",
			Size = UDim2.new(1, 0, 0, 0),
			AutomaticSize = Enum.AutomaticSize.Y,
			AutomaticCanvasSize = Enum.AutomaticSize.Y,
			CanvasSize = UDim2.new(0, 0, 0, 0),
			BackgroundTransparency = 1,
			BorderSizePixel = 0,
			ScrollBarThickness = 4,
			ScrollingDirection = Enum.ScrollingDirection.Y,
		}, function()
			Element("UISizeConstraint", {
				id = "clamp",
				MaxSize = Vector2.new(math.huge, maxListHeight),
			})
			Element("UIListLayout", {
				id = "layout",
				Padding = UDim.new(0, 6),
				SortOrder = Enum.SortOrder.LayoutOrder,
			})
			Element("UIPadding", {
				id = "pad",
				PaddingRight = UDim.new(0, 6),
			})

			for _, conflict in conflicts do
				ConflictRow(conflict)
			end
		end)

		local canComplete = unresolved == 0 and not completing
		Element("TextButton", {
			id = "complete",
			Text = if completing
				then "Completing…"
				elseif completeError then "Complete failed — retry"
				elseif unresolved > 0 then `Complete ({unresolved} undecided)`
				else "Complete",
			Size = UDim2.new(1, 0, 0, 30),
			BackgroundColor3 = if canComplete then Color3.fromRGB(60, 160, 90) else Color3.fromRGB(55, 55, 60),
			TextColor3 = Color3.fromRGB(240, 240, 240),
			TextSize = 14,
			Font = Enum.Font.GothamMedium,
			events = {
				Activated = function()
					if canComplete then
						complete()
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

-- Self-terminating render loop: if this session's panel disappears (a newer
-- resolver run's boot cleanup destroyed it, or the run was killed), stop
-- rendering — a killed run's Heartbeat connection otherwise survives and
-- errors every frame trying to reparent a destroyed ScreenGui.
local screenGui: Instance? = nil
local connection
connection = RunService.Heartbeat:Connect(function()
	if screenGui and screenGui.Parent == nil then
		connection:Disconnect()
		finished = true
		return
	end
	updateHighlights()
	Portal(CoreGui, "rbx-diff-resolver", function()
		local guiNode = Element("ScreenGui", {
			id = "gui",
			Name = "RbxDiffResolver",
			ResetOnSpawn = false,
			DisplayOrder = 200,
			ZIndexBehavior = Enum.ZIndexBehavior.Sibling,
		}, function()
			Panel()
		end)
		screenGui = (guiNode :: any).instance
	end)
end)

-- Debug drive-through: stage everything to one side and Complete, so the
-- whole pipeline can run headlessly (rodeo run ... -- <file> --auto ours)
if args.autoSide then
	for _, conflict in conflicts do
		conflict.resolved = args.autoSide
	end
	complete()
end

while not finished do
	task.wait(0.25)
end
task.wait(2) -- let the success state render briefly

connection:Disconnect()
for _, highlight in liveHighlights do
	highlight:Destroy()
end

return `finalized {#conflicts} conflict(s) — {mergedFile} is clean`
