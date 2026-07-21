local packages = require("../packages")
local Element = packages.miu.Element

local format = require("../library/format")
local theme = require("../library/theme")

local conflicts = require("../ecs/systems/library/conflicts")
local cts = require("../ecs/cts")
local world = require("../ecs/world")

local Button = require("./Button")
local Label = require("./Label")

local jecs = packages.jecs

local function ConflictRow(entity: any)
	local name = world:get(entity, jecs.Name) :: string
	local kind = world:get(entity, cts.Kind) :: string
	local path = world:get(entity, cts.Path) :: string
	local property = world:get(entity, cts.Property) :: string?
	local resolution = world:get(entity, cts.Resolution) :: string?
	local target = world:get(entity, cts.Target) :: Instance
	local selected = world:get(cts.SelectedConflict, cts.SelectedConflict) == entity

	-- Fixed-height card that fills its VirtualList slot; every line truncates
	-- instead of wrapping so the row height stays uniform.
	Element("TextButton", {
		id = "card",
		Text = "",
		AutoButtonColor = false,
		Size = UDim2.fromScale(1, 1),
		tags = { "flex-col", "gap-sm", "p-md", "corner-sm", "vs-border" },
		BackgroundColor3 = if selected then theme.surface:Lerp(theme.selection, 0.35) else theme.surface,
		events = {
			Activated = function()
				world:set(cts.SelectedConflict, cts.SelectedConflict, entity)
				world:set(cts.SelectedInstance, cts.SelectedInstance, target)
			end,
		},
	}, function()
		Label("path", path, theme.statusColor(resolution), true)

		local detail = kind .. (if property then ` ({property})` else "")
		Element("TextLabel", {
			id = "detail",
			Text = detail,
			tags = { "text", "text-small", "muted", "auto-size-y" },
			TextTruncate = Enum.TextTruncate.AtEnd,
		})

		local oursSummary = format.sideSummary(kind, property, world:get(entity, cts.Ours))
		local theirsSummary = format.sideSummary(kind, property, world:get(entity, cts.Theirs))
		if oursSummary or theirsSummary then
			Element("TextLabel", {
				id = "values",
				RichText = true,
				Text = `<font color="{theme.oursHex}">{oursSummary or "?"}</font>  ·  <font color="{theme.theirsHex}">{theirsSummary or "?"}</font>`,
				tags = { "text", "text-small", "auto-size-y" },
				TextTruncate = Enum.TextTruncate.AtEnd,
			})
		end

		Element("Frame", {
			id = "buttons",
			Size = UDim2.new(1, 0, 0, 22),
			tags = { "bg-none", "flex-row", "gap-sm" },
		}, function()
			Button("ours", "Ours", resolution == "ours", theme.ours, function()
				conflicts.choose(entity, "ours")
			end)
			Button("theirs", "Theirs", resolution == "theirs", theme.theirs, function()
				conflicts.choose(entity, "theirs")
			end)
			if conflicts.canCustom(entity) then
				Button("custom", "Custom", resolution == "custom", theme.custom, function()
					conflicts.choose(entity, "custom")
				end)
			end
			if target:IsA("PVInstance") and target:IsDescendantOf(workspace) then
				Button("view", "View", false, theme.neutralButton, function()
					world:set(world:entity(), cts.FocusRequest, entity)
				end)
			end
		end)
	end)
end

return ConflictRow
