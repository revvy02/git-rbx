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

	Layout: the ecs/ world is the source of truth — conflict entities +
	session singletons (ecs/cts.luau), shared query/staging APIs in
	ecs/library/, and self-registering systems in ecs/systems/ (each returns
	function(world, cts, scheduler) and calls scheduler:addSystem itself).
	config.luau (launch config), format.luau (value/path text), transport.luau
	(Complete → CLI), components/ (miu panels: Explorer + PropertiesPanel in
	the sidebar widget, Panel with ConflictRows in the conflicts widget).

	Args: <merged-file> [--rbx-diff <path-to-binary>] [--auto <side>]
]]

local process = require("@rodeo/process")

local argparse = require("@pkg/argparse")

-- Real packages, not vendors: packages.luau rojo-builds roblox_packages and
-- imports it, so the pesde shims resolve as instances.
local packages = require("@self/packages")
local Element = packages.miu.Instance
local Portal = packages.miu.Portal

local StyleLink = packages.wind.StyleLink
local StyleSheet = packages.wind.StyleSheet
local WindRules = packages.wind.WindRules

local config = require("@self/config")
local preview = require("@self/ecs/library/preview")
local theme = require("@self/theme")
local transport = require("@self/transport")

local cts = require("@self/ecs/cts")
local ecsConflicts = require("@self/ecs/library/conflicts")
local scheduler = require("@self/ecs/scheduler")
local world = require("@self/ecs/world")

local Explorer = require("@self/components/Explorer")
local Panel = require("@self/components/Panel")
local PropertiesPanel = require("@self/components/PropertiesPanel")

local CONTAINER_NAME = "__RbxDiffMerge"

local parser = argparse("rbx-diff-resolver", "Resolve rbx-diff merge conflicts visually in Studio")
parser:argument("file", "Conflicted merged file written by `rbx-diff merge`")
parser:option("--rbx-diff", "Path to the rbx-diff binary"):target("rbxDiffBin"):default("rbx-diff")
parser:option("--auto", "Debug: stage every conflict to this side and Complete immediately"):target("autoSide")
parser:option("--resolver-root", "Resolver checkout dir (read by packages.luau)"):target("resolverRoot"):default(".")

local args = parser:parse(process.args)
local mergedFile = args.file

-- ============================================================================
-- Conflict model: read once from the DOM into conflict entities — from here
-- on the world is the source of truth for merge + session state.
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

local records: { ecsConflicts.ConflictRecord } = {}
for _, entry in container:GetChildren() do
	table.insert(records, {
		name = entry.Name,
		kind = entry:GetAttribute("Kind") :: string,
		path = entry:GetAttribute("Path") :: string,
		property = entry:GetAttribute("Property") :: string?,
		resolved = entry:GetAttribute("Resolved") :: string?,
		target = (entry:FindFirstChild("Target") :: ObjectValue).Value :: Instance,
		entry = entry,
	})
end
table.sort(records, function(a, b)
	return a.name < b.name
end)

config.mergedFile = mergedFile
config.rbxDiffBin = args.rbxDiffBin

ecsConflicts.boot(records)
preview.boot(container :: Instance)

-- Session singletons: a component set on its own entity id.
world:set(cts.ViewTab, cts.ViewTab, "result")
local firstConflict = ecsConflicts.all()[1]
if firstConflict then
	world:set(cts.SelectedConflict, cts.SelectedConflict, firstConflict)
	world:set(cts.SelectedInstance, cts.SelectedInstance, world:get(firstConflict, cts.Target))
end

print(`rbx-diff resolver: {#records} conflict(s) in {mergedFile}`)

-- ============================================================================
-- Sidebar host: a real plugin dock widget so the explorer/props don't
-- overlap the viewport. rodeo runs us at plugin identity but hands us no
-- `plugin` global — the rodeo plugin itself is a Plugin instance under
-- PluginDebugService, and CreateDockWidgetPluginGuiAsync works from here.
-- ============================================================================

local rodeoPlugin: Plugin?
for _, child in game:GetService("PluginDebugService"):GetChildren() do
	if child:IsA("Plugin") and child.Name:match("rodeo") then
		rodeoPlugin = child
		break
	end
end

local function createWidget(id: string, title: string, dockState: Enum.InitialDockState): DockWidgetPluginGui
	local widget = (rodeoPlugin :: Plugin):CreateDockWidgetPluginGuiAsync(
		id,
		DockWidgetPluginGuiInfo.new(dockState, true, true, 300, 600, 240, 300)
	)
	widget.Title = title
	return widget
end

local sidebarWidget = createWidget("RbxDiffMergeExplorer", "rbx-diff", Enum.InitialDockState.Left)
local conflictsWidget = createWidget("RbxDiffMergeConflicts", "rbx-diff — conflicts", Enum.InitialDockState.Right)

-- ============================================================================
-- Systems; the run stays alive until finalized
-- ============================================================================

local function Sidebar()
	-- Explorer top half, properties bottom half.
	Element("Frame", {
		id = "sidebar",
		Size = UDim2.new(1, 0, 1, 0),
		tags = { "surface", "vs-border" },
	}, function()
		Explorer()
		PropertiesPanel()
	end)
end

local function ResolverSheet()
	-- Each render surface needs its own sheet link: the wind utility classes
	-- (flex-*, gap-*, p-*, corner-*, auto-size-*, bg-none, ...) plus the
	-- resolver's VS Code-dark classes from theme.rules().
	StyleLink(StyleSheet(function()
		WindRules()
		theme.rules()
	end))
end

local Heartbeat = packages.planckRunService.Phases.Heartbeat

-- ECS systems self-register on the scheduler (same-phase order = add order:
-- previews render, the mirror diffs the DOM, highlights re-adorn, UI last).
for _, registerSystem in
	{
		require("@self/ecs/systems/renderPreviews"),
		require("@self/ecs/systems/mirror"),
		require("@self/ecs/systems/updateHighlights"),
	}
do
	registerSystem(world, cts, scheduler)
end
scheduler:addSystem({
	phase = Heartbeat,
	system = function()
		Portal(sidebarWidget, "rbx-diff-sidebar", function()
			ResolverSheet()
			Sidebar()
		end)
	end,
})
scheduler:addSystem({
	phase = Heartbeat,
	system = function()
		Portal(conflictsWidget, "rbx-diff-conflicts", function()
			ResolverSheet()
			Element("Frame", {
				id = "conflicts",
				Size = UDim2.new(1, 0, 1, 0),
				tags = { "surface", "vs-border" },
			}, function()
				Panel()
			end)
		end)
	end,
})

-- Debug drive-through: stage everything to one side and Complete, so the
-- whole pipeline can run headlessly (rodeo run ... -- <file> --auto ours)
if args.autoSide then
	for _, entity in ecsConflicts.all() do
		world:set(entity, cts.Resolution, args.autoSide)
	end
	transport.complete()
end

while not world:has(cts.Finished, cts.Finished) do
	task.wait(0.25)
end

return `finalized {#records} conflict(s) — {mergedFile} is clean`
