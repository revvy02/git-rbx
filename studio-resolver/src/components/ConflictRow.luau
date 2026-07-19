local Element = require("../packages").miu.Instance

local format = require("../format")
local preview = require("../ecs/systems/preview")
local state = require("../state")
local theme = require("../theme")

local Button = require("./Button")
local Label = require("./Label")

type Conflict = state.Conflict

local function ConflictRow(conflict: Conflict)
	Element("TextButton", {
		id = conflict.name,
		Text = "",
		AutoButtonColor = false,
		tags = { "auto-size-y", "flex-col", "gap-sm", "p-md", "corner-sm", "vs-border" },
		BackgroundColor3 = if state.selected == conflict
			then theme.surface:Lerp(theme.selection, 0.35)
			else theme.surface,
		events = {
			Activated = function()
				state.selected = conflict
				state.selectedInstance = conflict.target
			end,
		},
	}, function()
		Label("path", conflict.path, state.statusColor(conflict))

		local detail = conflict.kind .. (if conflict.property then ` ({conflict.property})` else "")
		Element("TextLabel", {
			id = "detail",
			Text = detail,
			tags = { "text-base", "auto-size-y" },
			TextColor3 = theme.textMuted,
			TextXAlignment = Enum.TextXAlignment.Left,
			TextSize = theme.textSizeSmall,
			Font = theme.font,
			TextWrapped = true,
		})

		local oursSummary = format.sideSummary(conflict, "ours")
		local theirsSummary = format.sideSummary(conflict, "theirs")
		if oursSummary or theirsSummary then
			Element("TextLabel", {
				id = "values",
				RichText = true,
				Text = `<font color="{theme.oursHex}">{oursSummary or "?"}</font>  ·  <font color="{theme.theirsHex}">{theirsSummary or "?"}</font>`,
				tags = { "text-base", "auto-size-y" },
				TextColor3 = theme.text,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextSize = theme.textSizeSmall,
				Font = theme.font,
				TextWrapped = true,
			})
		end

		Element("Frame", {
			id = "buttons",
			Size = UDim2.new(1, 0, 0, 22),
			tags = { "bg-none", "flex-row", "gap-sm" },
		}, function()
			Button("ours", "Ours", conflict.resolved == "ours", state.OURS_COLOR, function()
				state.choose(conflict, "ours")
			end)
			Button("theirs", "Theirs", conflict.resolved == "theirs", state.THEIRS_COLOR, function()
				state.choose(conflict, "theirs")
			end)
			if state.canCustom(conflict) then
				Button("custom", "Custom", conflict.resolved == "custom", state.CUSTOM_COLOR, function()
					state.choose(conflict, "custom")
				end)
			end
			if conflict.target:IsA("PVInstance") and conflict.target:IsDescendantOf(workspace) then
				Button("view", "View", false, theme.neutralButton, function()
					preview.focusOn(conflict)
				end)
			end
		end)
	end)
end

return ConflictRow
