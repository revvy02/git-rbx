local packages = require("../packages")
local Element = packages.miu.Element

local format = require("../library/format")
local theme = require("../library/theme")

local conflicts = require("../ecs/systems/library/conflicts")
local cts = require("../ecs/cts")
local world = require("../ecs/world")

local Button = require("./Button")
local Label = require("./Label")

-- One conflict — or one rigid group, which reads member state and stages
-- every member at once.
local function ConflictRow(entity: any)
	local kind = world:get(entity, cts.Kind) :: string
	local path = world:get(entity, cts.Path) :: string
	local property = world:get(entity, cts.Property) :: string?
	local target = world:get(entity, cts.Target) :: Instance?
	local members = world:get(entity, cts.Members) :: { any }?
	local resolution = if members
		then conflicts.groupResolution(entity)
		else world:get(entity, cts.Resolution) :: string?
	local selected = world:get(cts.SelectedConflict, cts.SelectedConflict) == entity
	local viewable = target ~= nil and target:IsA("PVInstance") and target:IsDescendantOf(workspace)

	local function requestFocus()
		if viewable then
			world:set(world:entity(), cts.FocusRequest, entity)
		end
	end

	-- Staging a side also frames it — picking a decision and seeing the
	-- result are one gesture. The focus lands after the preview applies (the
	-- previews system runs before focus in the frame), so the camera fits
	-- whatever the choice renders.
	local function choose(side: string)
		if members then
			conflicts.chooseGroup(entity, side)
		else
			conflicts.choose(entity, side)
		end
		requestFocus()
	end

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
				if target then
					world:set(cts.SelectedInstance, cts.SelectedInstance, target)
				end
			end,
		},
	}, function()
		Label("path", path, theme.statusColor(resolution), true)

		local detail = if members
			then `{kind} — {#members} spatial changes`
			else kind .. (if property then ` ({property})` else "")
		Element("TextLabel", {
			id = "detail",
			Text = detail,
			tags = { "text", "text-small", "muted", "auto-size-y" },
			TextTruncate = Enum.TextTruncate.AtEnd,
		})

		local oursSummary: string?
		local theirsSummary: string?
		if members then
			local deltaOurs = world:get(entity, cts.DeltaOurs) :: CFrame?
			local deltaTheirs = world:get(entity, cts.DeltaTheirs) :: CFrame?
			oursSummary = if deltaOurs then format.formatDelta(deltaOurs) else nil
			theirsSummary = if deltaTheirs then format.formatDelta(deltaTheirs) else nil
		else
			oursSummary = format.sideSummary(kind, property, world:get(entity, cts.Ours))
			theirsSummary = format.sideSummary(kind, property, world:get(entity, cts.Theirs))
		end
		if oursSummary or theirsSummary then
			Element("Frame", {
				id = "values",
				Size = UDim2.new(1, 0, 0, 28),
				tags = { "bg-none", "flex-col" },
			}, function()
				Element("TextLabel", {
					id = "ours",
					RichText = true,
					Text = oursSummary or "?",
					Size = UDim2.new(1, 0, 0, 14),
					tags = { "text", "text-small" },
					TextColor3 = theme.ours,
					TextTruncate = Enum.TextTruncate.AtEnd,
				})
				Element("TextLabel", {
					id = "theirs",
					RichText = true,
					Text = theirsSummary or "?",
					Size = UDim2.new(1, 0, 0, 14),
					tags = { "text", "text-small" },
					TextColor3 = theme.theirs,
					TextTruncate = Enum.TextTruncate.AtEnd,
				})
			end)
		end

		Element("Frame", {
			id = "buttons",
			Size = UDim2.new(1, 0, 0, 22),
			tags = { "bg-none", "flex-row", "gap-sm" },
		}, function()
			Button("ours", "Ours", resolution == "ours", theme.ours, function()
				choose("ours")
			end)
			Button("theirs", "Theirs", resolution == "theirs", theme.theirs, function()
				choose("theirs")
			end)
			if not members and conflicts.canCustom(entity) then
				Button("custom", "Custom", resolution == "custom", theme.custom, function()
					choose("custom")
				end)
			end
			if viewable then
				Button("view", "View", false, theme.neutralButton, requestFocus)
			end
		end)
	end)
end

return ConflictRow
