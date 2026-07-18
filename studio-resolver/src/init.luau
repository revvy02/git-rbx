--[[
	rbx-diff studio resolver — visual front end over the in-file conflict state.

	Run against the conflicted file (places open directly; models import
	into a preview folder). `rbx-diff resolve <file> --studio` launches this
	from a copy of this tree embedded in the binary; from the checkout:

	  rodeo run --place ./merged.rbxl --focus src/init.luau -- ./merged.rbxl
	  rodeo run --place --focus src/init.luau -- ./merged.rbxm

	The run owns the Studio it spawns, so Studio closes itself when the
	session completes — pass --detach to keep it open.

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

	Args: <merged-file> [--rbx-diff <path-to-binary>] [--auto <side>]
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

local StyleLink = require("@self/wind/StyleLink")
local StyleSheet = require("@self/wind/StyleSheet")
local WindRules = require("@self/wind/WindRules")

local getBoundingBox = require("@self/viewport/getBoundingBox")
local getIdealCameraCFrame = require("@self/viewport/getIdealCameraCFrame")

local CONTAINER_NAME = "__RbxDiffMerge"

local UNRESOLVED_COLOR = Color3.fromRGB(255, 170, 0)
local OURS_COLOR = Color3.fromRGB(70, 160, 255)
local THEIRS_COLOR = Color3.fromRGB(170, 90, 255)
local CUSTOM_COLOR = Color3.fromRGB(80, 200, 160)

local parser = argparse("rbx-diff-resolver", "Resolve rbx-diff merge conflicts visually in Studio")
parser:argument("file", "Conflicted merged file written by `rbx-diff merge`")
parser:option("--rbx-diff", "Path to the rbx-diff binary"):target("rbxDiffBin"):default("rbx-diff")
parser:option("--auto", "Debug: stage every conflict to this side and Complete immediately"):target("autoSide")

local args = parser:parse(process.args)
local mergedFile = args.file
local rbxDiffBin = args.rbxDiffBin

-- ============================================================================
-- Conflict model: read once from the DOM. Resolution state is kept locally
-- and mirrored onto the live entries so other in-Studio tools see it too.
-- ============================================================================

type SideInfo = {
	clone: Instance?, -- this side's version (nil = this side deletes the target)
	destination: Instance?, -- MoveTarget: the parent this side moves the target to
	destinationPath: string?,
}

type Conflict = {
	name: string,
	kind: string,
	path: string,
	property: string?,
	resolved: string?,
	target: Instance,
	entry: Instance,

	-- 3D preview state, filled by the rendering boot pass
	ours: SideInfo?,
	theirs: SideInfo?,
	baseValue: any, -- Property: value to restore on unpick
	baseParent: Instance?, -- MoveTarget: parent to restore on unpick
	targetParts: { [BasePart]: number }?, -- DeleteVsEdit: true transparencies
	rendered: string?, -- staged state the 3D layer currently reflects
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
	table.insert(conflicts, {
		name = entry.Name,
		kind = entry:GetAttribute("Kind") :: string,
		path = entry:GetAttribute("Path") :: string,
		property = entry:GetAttribute("Property") :: string?,
		resolved = entry:GetAttribute("Resolved") :: string?,
		target = (entry:FindFirstChild("Target") :: ObjectValue).Value :: Instance,
		entry = entry,
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
	elseif conflict.resolved == "custom" then
		return CUSTOM_COLOR
	end
	return UNRESOLVED_COLOR
end

-- The live value of the conflicted property on the (copy) target instance.
local function liveValue(conflict: Conflict): any
	if conflict.property == "Name" then
		return conflict.target.Name
	end
	return (conflict.target :: any)[conflict.property :: string]
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
	return conflict.kind == "Property" and encodeValue(liveValue(conflict)) ~= nil
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
				local encoded = encodeValue(liveValue(conflict))
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
			local ok, result = pcall(process.run, args)
			if not ok or not result.ok then
				completeError = `--take {conflict.name} failed: {if ok then result.err else result}`
				warn(completeError)
				completing = false
				return
			end
		end
		local ok, result = pcall(process.run, { rbxDiffBin, "resolve", mergedFile, "--finalize" })
		if ok and result.ok then
			finished = true
		else
			completeError = `finalize failed: {if ok then result.err else result}`
			warn(completeError)
		end
		completing = false
	end)
end

-- Frame the whole target in view — fitted distance from its bounding box and
-- the live camera FOV/aspect (vendored miu-viewport math), not a fixed offset
-- that lands inside or behind geometry depending on target size. Focus is the
-- edit camera's orbit pivot: it must be written together with the CFrame, or
-- Studio keeps steering around the stale pivot and the view drifts off target.
local function focusOn(conflict: Conflict)
	local camera = workspace.CurrentCamera
	local cframe, size = getBoundingBox(conflict.target :: PVInstance)
	camera.CFrame = getIdealCameraCFrame(cframe, size, {
		fov = camera.FieldOfView,
		aspectRatio = camera.ViewportSize.X / camera.ViewportSize.Y,
	})
	camera.Focus = cframe
end

-- ============================================================================
-- 3D rendering: status highlight on the target, ghost twins of both sides
-- (spawned from the stamped Ours/Theirs clones), and a live preview — staging
-- a side morphs the Studio COPY toward that outcome (property values, parents,
-- delete fades) and unpicking restores the boot-time base state. None of this
-- touches the disk file; the CLI at Complete is still the only writer.
-- Ghost + status highlights each cost a Highlight instance (engine cap ~31
-- concurrent) — fine at typical conflict counts, revisit for huge merges.
-- ============================================================================

local staleGhosts = workspace:FindFirstChild("__RbxDiffGhosts")
if staleGhosts then
	staleGhosts:Destroy()
end
local ghostRoot = Instance.new("Folder")
ghostRoot.Name = "__RbxDiffGhosts"
ghostRoot.Archivable = false
ghostRoot.Parent = workspace

type Ghost = {
	root: Instance,
	parts: { [BasePart]: number }, -- part -> the clone's true transparency
	highlight: Highlight,
	truePivot: CFrame, -- where this version actually lives
	flankPivot: CFrame, -- parked beside the target for comparison
}

local ghosts: { [string]: Ghost } = {} -- key: `{conflict.name}/{side}`
local destinationHighlights: { [string]: Highlight } = {} -- MoveTarget, same key

local function eachPart(root: Instance, fn: (BasePart) -> ())
	if root:IsA("BasePart") then
		fn(root)
	end
	for _, descendant in root:GetDescendants() do
		if descendant:IsA("BasePart") then
			fn(descendant)
		end
	end
end

local function sideInfo(entry: Instance, sideName: string): SideInfo
	local info: SideInfo = {}
	local folder = entry:FindFirstChild(sideName)
	if folder then
		for _, child in folder:GetChildren() do
			if child.Name == "Destination" and child:IsA("ObjectValue") then
				info.destination = child.Value
			else
				info.clone = child
			end
		end
		info.destinationPath = folder:GetAttribute("DestinationPath") :: string?
	end
	return info
end

local function makeGhost(conflict: Conflict, side: string, clone: Instance, flankOffset: Vector3): Ghost?
	if not clone:IsA("PVInstance") then
		return nil
	end
	local root = clone:Clone()
	root.Name = `{conflict.name}_{side}`
	local parts: { [BasePart]: number } = {}
	eachPart(root, function(part)
		parts[part] = part.Transparency
		part.Anchored = true
		part.CanCollide = false
		part.CanQuery = false
		part.CanTouch = false
		part.Locked = true
	end)
	local color = if side == "ours" then OURS_COLOR else THEIRS_COLOR
	local highlight = Instance.new("Highlight")
	highlight.FillColor = color
	highlight.OutlineColor = color
	highlight.Adornee = root
	highlight.Parent = root
	local truePivot = (clone :: PVInstance):GetPivot()
	root.Parent = ghostRoot
	local ghost: Ghost = {
		root = root,
		parts = parts,
		highlight = highlight,
		truePivot = truePivot,
		flankPivot = truePivot + flankOffset,
	}
	(root :: PVInstance):PivotTo(ghost.flankPivot)
	return ghost
end

-- "dim": translucent reference beside the target. "solid": the true look, for
-- a version previewed in place. "hidden": the target itself shows it already.
local function setGhostState(ghost: Ghost, state: "dim" | "solid" | "hidden", pivot: CFrame?)
	for part, original in ghost.parts do
		part.Transparency = if state == "solid" then original elseif state == "dim" then 1 - (1 - original) * 0.35 else 1
	end
	ghost.highlight.Enabled = state ~= "hidden"
	ghost.highlight.FillTransparency = if state == "solid" then 0.8 else 0.92
	ghost.highlight.OutlineTransparency = if state == "solid" then 0 else 0.5
	if pivot then
		(ghost.root :: PVInstance):PivotTo(pivot)
	end
end

-- Boot pass: side clones, base snapshots (for unpick restore), ghost twins.
-- Ghosts spawn for spatial targets only, and not for Name conflicts (both
-- twins would look identical — the panel summary carries those).
for _, conflict in conflicts do
	conflict.ours = sideInfo(conflict.entry, "Ours")
	conflict.theirs = sideInfo(conflict.entry, "Theirs")
	conflict.rendered = "__init"

	local target = conflict.target
	if conflict.kind == "Property" then
		conflict.baseValue = liveValue(conflict)
	elseif conflict.kind == "MoveTarget" then
		conflict.baseParent = target.Parent
	elseif conflict.kind == "DeleteVsEdit" then
		local parts: { [BasePart]: number } = {}
		eachPart(target, function(part)
			parts[part] = part.Transparency
		end)
		conflict.targetParts = parts
	end

	local spatial = target:IsA("PVInstance") and target:IsDescendantOf(workspace)
	if spatial and (conflict.kind == "Property" and conflict.property ~= "Name" or conflict.kind == "DeleteVsEdit") then
		local _, size = getBoundingBox(target :: PVInstance)
		local gap = size.X + 3
		for side, offset in { ours = Vector3.new(-gap, 0, 0), theirs = Vector3.new(gap, 0, 0) } :: { [string]: Vector3 } do
			local clone = ((if side == "ours" then conflict.ours else conflict.theirs) :: SideInfo).clone
			if clone then
				local ghost = makeGhost(conflict, side, clone, offset)
				if ghost then
					ghosts[`{conflict.name}/{side}`] = ghost
				end
			end
		end
	end
	if conflict.kind == "MoveTarget" then
		for side, info in
			{ ours = conflict.ours :: SideInfo, theirs = conflict.theirs :: SideInfo } :: { [string]: SideInfo }
		do
			local destination = info.destination
			if destination and destination:IsA("PVInstance") and destination:IsDescendantOf(workspace) then
				local color = if side == "ours" then OURS_COLOR else THEIRS_COLOR
				local highlight = Instance.new("Highlight")
				highlight.FillColor = color
				highlight.OutlineColor = color
				highlight.FillTransparency = 0.92
				highlight.OutlineTransparency = 0.5
				highlight.Adornee = destination
				highlight.Parent = ghostRoot
				destinationHighlights[`{conflict.name}/{side}`] = highlight
			end
		end
	end
end

-- Morph the Studio copy toward the staged outcome; nil (unpicked) restores
-- the base snapshot. Ghost emphasis follows: the staged side's ghost hides
-- (the target now previews it), the rejected side stays dim for comparison.
local function applyPreview(conflict: Conflict)
	local staged = conflict.resolved

	if conflict.kind == "Property" then
		if staged ~= "custom" then
			local sideClone = if staged == "ours"
				then (conflict.ours :: SideInfo).clone
				elseif staged == "theirs" then (conflict.theirs :: SideInfo).clone
				else nil
			local property = conflict.property :: string
			if sideClone then
				(conflict.target :: any)[property] = (sideClone :: any)[property]
			elseif staged == nil then
				(conflict.target :: any)[property] = conflict.baseValue
			end
		end
	elseif conflict.kind == "MoveTarget" then
		local destination = if staged == "ours"
			then (conflict.ours :: SideInfo).destination
			elseif staged == "theirs" then (conflict.theirs :: SideInfo).destination
			else nil
		-- A side with no live destination (moved into the other branch's added
		-- content) previews as base rather than sticking at a stale parent.
		destination = destination or conflict.baseParent
		if destination then
			conflict.target.Parent = destination
		end
	elseif conflict.kind == "DeleteVsEdit" then
		-- Staging the delete side fades the target to a remnant; staging the
		-- edit side swaps the edited ghost in solid at the true position and
		-- hides the base target entirely.
		local editSide = if (conflict.ours :: SideInfo).clone then "ours" else "theirs"
		local fade = if staged == nil then nil elseif staged == editSide then 1 else 0.92
		for part, original in conflict.targetParts :: { [BasePart]: number } do
			part.Transparency = fade or original
		end
		local ghost = ghosts[`{conflict.name}/{editSide}`]
		if ghost then
			if staged == editSide then
				setGhostState(ghost, "solid", ghost.truePivot)
			else
				setGhostState(ghost, "dim", ghost.flankPivot)
			end
		end
		return
	end

	for _, side in { "ours", "theirs" } do
		local ghost = ghosts[`{conflict.name}/{side}`]
		if ghost then
			setGhostState(ghost, if staged == side then "hidden" else "dim")
		end
		local highlight = destinationHighlights[`{conflict.name}/{side}`]
		if highlight then
			highlight.FillTransparency = if staged == side then 0.75 else 0.92
			highlight.OutlineTransparency = if staged == side then 0 else 0.5
		end
	end
end

local function renderPreviews()
	if finished then
		if ghostRoot.Parent then
			ghostRoot:Destroy()
		end
		return
	end
	for _, conflict in conflicts do
		local state = conflict.resolved or "unresolved"
		if conflict.rendered ~= state then
			conflict.rendered = state
			applyPreview(conflict)
		end
	end
end

local liveHighlights: { [string]: Highlight } = {}

local function updateHighlights()
	for _, conflict in conflicts do
		local target = conflict.target
		local spatial = target:IsDescendantOf(workspace) and target:IsA("PVInstance")
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
		tags = { "text-base", "auto-size-y" },
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
		tags = { "corner-sm" },
		BackgroundColor3 = if highlighted then color else Color3.fromRGB(60, 60, 65),
		TextColor3 = Color3.fromRGB(240, 240, 240),
		TextSize = 13,
		Font = Enum.Font.GothamMedium,
		AutoButtonColor = true,
		events = { Activated = onClick },
	})
end

local function escapeRich(s: string): string
	return (s:gsub("&", "&amp;"):gsub("<", "&lt;"):gsub(">", "&gt;"))
end

local function formatValue(v: any): string
	local t = typeof(v)
	if t == "Color3" then
		return `{math.round(v.R * 255)}, {math.round(v.G * 255)}, {math.round(v.B * 255)}`
	elseif t == "number" then
		return string.format("%.4g", v)
	elseif t == "string" then
		return `"{escapeRich(v)}"`
	elseif t == "boolean" then
		return tostring(v)
	end
	return t
end

-- What choosing this side means, one glance: the property value it sets, the
-- destination it moves to, or whether it keeps (edited) / deletes the target.
local function sideSummary(conflict: Conflict, side: string): string?
	local info = (if side == "ours" then conflict.ours else conflict.theirs) :: SideInfo?
	if not info then
		return nil
	end
	if conflict.kind == "Property" then
		local clone = info.clone
		return clone and formatValue((clone :: any)[conflict.property :: string])
	elseif conflict.kind == "MoveTarget" then
		local path = info.destinationPath
		return path and `→ {escapeRich(path)}`
	elseif conflict.kind == "DeleteVsEdit" then
		return if info.clone then "keep (edited)" else "delete"
	end
	return nil
end

local function ConflictRow(conflict: Conflict)
	Element("Frame", {
		id = conflict.name,
		tags = { "auto-size-y", "flex-col", "gap-sm", "p-md", "corner-md" },
		BackgroundColor3 = Color3.fromRGB(40, 40, 46),
	}, function()
		Label("path", conflict.path, statusColor(conflict))

		local detail = conflict.kind .. (if conflict.property then ` ({conflict.property})` else "")
		Element("TextLabel", {
			id = "detail",
			Text = detail,
			tags = { "text-base", "auto-size-y" },
			TextColor3 = Color3.fromRGB(160, 160, 170),
			TextXAlignment = Enum.TextXAlignment.Left,
			TextSize = 12,
			Font = Enum.Font.Gotham,
			TextWrapped = true,
		})

		local oursSummary = sideSummary(conflict, "ours")
		local theirsSummary = sideSummary(conflict, "theirs")
		if oursSummary or theirsSummary then
			Element("TextLabel", {
				id = "values",
				RichText = true,
				Text = `<font color="#46A0FF">{oursSummary or "?"}</font>  ·  <font color="#AA5AFF">{theirsSummary or "?"}</font>`,
				tags = { "text-base", "auto-size-y" },
				TextColor3 = Color3.fromRGB(200, 200, 205),
				TextXAlignment = Enum.TextXAlignment.Left,
				TextSize = 12,
				Font = Enum.Font.RobotoMono,
				TextWrapped = true,
			})
		end

		Element("Frame", {
			id = "buttons",
			Size = UDim2.new(1, 0, 0, 22),
			tags = { "bg-none", "flex-row", "gap-sm" },
		}, function()
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
			if conflict.target:IsA("PVInstance") and conflict.target:IsDescendantOf(workspace) then
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

	-- The scroller is the panel: invisible, stretched to the bottom of the
	-- screen; rows and the button carry their own backgrounds, and
	-- AutomaticCanvasSize extends past the viewport when content overflows.
	Element("ScrollingFrame", {
		id = "panel",
		Position = UDim2.new(1, -356, 0, 16),
		Size = UDim2.new(0, 340, 1, -32),
		tags = { "bg-none", "flex-col", "gap-sm", "pr-md" },
		ScrollingDirection = Enum.ScrollingDirection.Y,
		AutomaticCanvasSize = Enum.AutomaticSize.Y,
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = if finished
				then "Merge complete — file written"
				else `rbx-diff: {#conflicts - unresolved}/{#conflicts} decided`,
			Size = UDim2.new(1, 0, 0, 26),
			tags = { "corner-md" },
			BackgroundColor3 = Color3.fromRGB(28, 28, 32),
			TextColor3 = if finished then Color3.fromRGB(120, 220, 140) else Color3.fromRGB(230, 230, 230),
			TextSize = 14,
			Font = Enum.Font.GothamMedium,
		})

		if finished then
			return
		end

		for _, conflict in conflicts do
			ConflictRow(conflict)
		end

		local canComplete = unresolved == 0 and not completing
		Element("TextButton", {
			id = "complete",
			Text = if completing
				then "Completing…"
				elseif completeError then "Complete failed — retry"
				elseif unresolved > 0 then `Complete ({unresolved} undecided)`
				else "Complete",
			Size = UDim2.new(1, 0, 0, 30),
			tags = { "corner-md" },
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
		})
	end)
end

RunService.Heartbeat:Connect(function()
	renderPreviews()
	updateHighlights()
	Portal(CoreGui, "rbx-diff-resolver", function()
		Element("ScreenGui", {
			id = "gui",
			Name = "RbxDiffResolver",
			ResetOnSpawn = false,
			DisplayOrder = 200,
			ZIndexBehavior = Enum.ZIndexBehavior.Sibling,
		}, function()
			-- One sheet for the whole panel: the wind utility classes
			-- (flex-*, gap-*, p-*, corner-*, auto-size-*, bg-none, ...) that
			-- the elements below reference through `tags`.
			StyleLink(StyleSheet(function()
				WindRules()
			end))
			Panel()
		end)
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

return `finalized {#conflicts} conflict(s) — {mergedFile} is clean`
