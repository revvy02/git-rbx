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

	Layout: this file is the composition root — parse args, read the stamped
	container into conflict entities + singletons, register systems. The world
	is the source of truth; all behavior lives in ecs/systems/ as scheduled
	loops (each returns a register function taking world/cts/scheduler), with
	stateless helpers in ecs/systems/library/. library/ holds the non-ecs
	support modules (config, format, theme, transport, viewport math);
	ui/ holds the miu panels the ui systems render (ExplorerPanel/,
	PropertyPanel/, and the conflicts-panel pieces).

	Args: <merged-file> [--rbx-diff <path-to-binary>] [--auto <side>]
]]

local process = require("@rodeo/process")

local argparse = require("@pkg/argparse")

-- Real packages, not vendors: packages.luau rojo-builds roblox_packages and
-- imports it, so the pesde shims resolve as instances.
local packages = require("@self/packages")
local jecs = packages.jecs

local config = require("@self/library/config")
local transport = require("@self/library/transport")

local cts = require("@self/ecs/cts")
local scheduler = require("@self/ecs/scheduler")
local world = require("@self/ecs/world")

local CONTAINER_NAME = "__RbxDiffMerge"

local parser = argparse("rbx-diff-resolver", "Resolve rbx-diff merge conflicts visually in Studio")
parser:argument("file", "Conflicted merged file written by `rbx-diff merge`")
parser:option("--rbx-diff", "Path to the rbx-diff binary"):target("rbxDiffBin"):default("rbx-diff")
parser:option("--auto", "Debug: stage every conflict to this side and Complete immediately"):target("autoSide")
parser:option("--resolver-root", "Resolver checkout dir (read by packages.luau)"):target("resolverRoot"):default(".")

local args = parser:parse(process.args)
local mergedFile = args.file

-- ============================================================================
-- Boot: read the stamped container once into conflict entities + singletons —
-- from here on the world is the source of truth for merge + session state.
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

type ConflictRecord = {
	name: string,
	kind: string,
	path: string,
	property: string?,
	resolved: string?,
	target: Instance,
	entry: Instance,
}

local records: { ConflictRecord } = {}
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

local conflictEntities: { any } = {}
for index, record in records do
	local entity = world:entity()
	world:set(entity, jecs.Name, record.name)
	world:set(entity, cts.Order, index)
	world:set(entity, cts.Kind, record.kind)
	world:set(entity, cts.Path, record.path)
	world:set(entity, cts.Target, record.target)
	world:set(entity, cts.Entry, record.entry)
	if record.property then
		world:set(entity, cts.Property, record.property)
	end
	if record.resolved then
		world:set(entity, cts.Resolution, record.resolved)
	end
	table.insert(conflictEntities, entity)
end
world:set(cts.Conflicts, cts.Conflicts, conflictEntities)
world:set(cts.Container, cts.Container, container)

world:set(cts.ViewTab, cts.ViewTab, "result")
local firstConflict = conflictEntities[1]
if firstConflict then
	world:set(cts.SelectedConflict, cts.SelectedConflict, firstConflict)
	world:set(cts.SelectedInstance, cts.SelectedInstance, world:get(firstConflict, cts.Target))
end

print(`rbx-diff resolver: {#records} conflict(s) in {mergedFile}`)

-- ============================================================================
-- Systems; the run stays alive until finalized
-- ============================================================================

-- Same-phase run order = registration order: previews render, the mirror
-- diffs the DOM, highlights re-adorn, focus requests resolve, UI last.
for _, registerSystem in
	{
		require("@self/ecs/systems/renderPreviews"),
		require("@self/ecs/systems/mirror"),
		require("@self/ecs/systems/updateHighlights"),
		require("@self/ecs/systems/focus"),
		require("@self/ecs/systems/uiSidebar"),
		require("@self/ecs/systems/uiConflicts"),
		require("@self/ecs/systems/entitiesDestroy"),
	}
do
	registerSystem(world, cts, scheduler)
end

-- Debug drive-through: stage everything to one side and Complete, so the
-- whole pipeline can run headlessly (rodeo run ... -- <file> --auto ours)
if args.autoSide then
	for _, entity in conflictEntities do
		world:set(entity, cts.Resolution, args.autoSide)
	end
	transport.complete()
end

while not world:has(cts.Finished, cts.Finished) do
	task.wait(0.25)
end

return `finalized {#records} conflict(s) — {mergedFile} is clean`
