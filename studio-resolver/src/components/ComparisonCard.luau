local Element = require("../miu/Instance")

local format = require("../format")
local state = require("../state")
local theme = require("../theme")

local Label = require("./Label")
local OptionRow = require("./OptionRow")

-- base | ours | theirs for the selected conflict; clicking a row stages it
-- (base = unpick). When custom is staged, shows the live value Complete will
-- capture.
local function ComparisonCard()
	local conflict = state.selected
	if not conflict then
		return
	end

	Element("Frame", {
		id = "comparison",
		tags = { "auto-size-y", "flex-col", "gap-sm", "p-md", "corner-sm", "surface", "vs-border" },
	}, function()
		Label("path", conflict.path, state.statusColor(conflict))
		Element("TextLabel", {
			id = "kind",
			Text = conflict.kind .. (if conflict.property then ` ({conflict.property})` else ""),
			tags = { "text-base", "auto-size-y" },
			TextColor3 = theme.textMuted,
			TextXAlignment = Enum.TextXAlignment.Left,
			TextSize = theme.textSizeSmall,
			Font = theme.font,
		})

		OptionRow("base", "base", format.baseSummary(conflict), theme.textMuted, conflict.resolved == nil, function()
			state.stage(conflict, nil)
		end)
		OptionRow(
			"ours",
			"ours",
			format.sideSummary(conflict, "ours"),
			state.OURS_COLOR,
			conflict.resolved == "ours",
			function()
				state.stage(conflict, "ours")
			end
		)
		OptionRow(
			"theirs",
			"theirs",
			format.sideSummary(conflict, "theirs"),
			state.THEIRS_COLOR,
			conflict.resolved == "theirs",
			function()
				state.stage(conflict, "theirs")
			end
		)
		if conflict.resolved == "custom" then
			Label("custom", `custom — live: {format.formatValue(state.liveValue(conflict))}`, state.CUSTOM_COLOR)
		end
	end)
end

return ComparisonCard
