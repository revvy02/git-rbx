--[[
	rbx-diff studio resolver — visual front end over the in-file conflict state.

	Run against the conflicted file (places open directly; models import
	into a preview folder). `rbx-diff resolve <file> --studio` launches this
	tree straight from the checkout; by hand:

	  rodeo run --place ./merged.rbxl --focus src/init.luau -- ./merged.rbxl
	  rodeo run --place --focus src/init.luau -- ./merged.rbxm

	The run owns the Studio it spawns, so Studio closes itself when the
	session completes — pass --detach to keep it open.

	Discovery is entirely in-DOM (the __RbxDiffMerge container + RbxDiffConflict
	tags that `rbx-diff merge` stamped). Decisions are STAGED in the session —
	nothing touches the disk file until Complete, which batches the canonical
	CLI calls host-side via @rodeo/process:

	  rbx-diff resolve <file> --take <side> --entry <Conflict_N>   (per decision)
	  rbx-diff resolve <file> --finalize

	The Studio DOM is a COPY of the file — the disk file stays authoritative and
	only rbx-diff writes it, only at Complete. Abandoning the session leaves the
	conflicted file exactly as `rbx-diff merge` wrote it.

	Layout: state.luau (shared model + staging), format.luau (value/path text),
	transport.luau (Complete → CLI), preview.luau (3D: select-renders preview,
	highlights, camera), components/ (miu panels: Explorer + ComparisonCard on
	the left, Panel with ConflictRows on the right).

	Args: <merged-file> [--rbx-diff <path-to-binary>] [--auto <side>]
]]

local CoreGui = game:GetService("CoreGui")
local RunService = game:GetService("RunService")

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

local preview = require("@self/preview")
local state = require("@self/state")
local theme = require("@self/theme")
local transport = require("@self/transport")

local Explorer = require("@self/components/Explorer")
local Panel = require("@self/components/Panel")

local CONTAINER_NAME = "__RbxDiffMerge"

local parser = argparse("rbx-diff-resolver", "Resolve rbx-diff merge conflicts visually in Studio")
parser:argument("file", "Conflicted merged file written by `rbx-diff merge`")
parser:option("--rbx-diff", "Path to the rbx-diff binary"):target("rbxDiffBin"):default("rbx-diff")
parser:option("--auto", "Debug: stage every conflict to this side and Complete immediately"):target("autoSide")

local args = parser:parse(process.args)
local mergedFile = args.file

-- ============================================================================
-- Conflict model: read once from the DOM into the shared state. Resolution
-- state is kept locally and mirrored onto the live entries so other in-Studio
-- tools see it too.
-- ============================================================================

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
	local previewFolder = Instance.new("Folder")
	previewFolder.Name = "RbxDiffModelPreview"
	for _, root in roblox.import(mergedFile) do
		root.Parent = previewFolder
	end
	previewFolder.Parent = workspace
	container = game:FindFirstChild(CONTAINER_NAME, true)
end

if not container then
	warn(`no {CONTAINER_NAME} container in this place — nothing to resolve`)
	return "no-conflicts"
end

local conflicts: { state.Conflict } = {}
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

state.conflicts = conflicts
state.selected = conflicts[1]
state.mergedFile = mergedFile
state.rbxDiffBin = args.rbxDiffBin

preview.boot(container :: Instance)

print(`rbx-diff resolver: {#conflicts} conflict(s) in {mergedFile}`)

-- ============================================================================
-- Frame loop; the run stays alive until finalized
-- ============================================================================

RunService.Heartbeat:Connect(function()
	preview.renderPreviews()
	preview.updateHighlights()
	Portal(CoreGui, "rbx-diff-resolver", function()
		Element("ScreenGui", {
			id = "gui",
			Name = "RbxDiffResolver",
			ResetOnSpawn = false,
			DisplayOrder = 200,
			ZIndexBehavior = Enum.ZIndexBehavior.Sibling,
		}, function()
			-- One sheet for the whole overlay: the wind utility classes
			-- (flex-*, gap-*, p-*, corner-*, auto-size-*, bg-none, ...) plus
			-- the resolver's VS Code-dark classes from theme.rules().
			StyleLink(StyleSheet(function()
				WindRules()
				theme.rules()
			end))
			Explorer()
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
	transport.complete()
end

while not state.finished do
	task.wait(0.25)
end

return `finalized {#conflicts} conflict(s) — {mergedFile} is clean`
