local Element = require("../miu/Instance")

local format = require("../format")
local preview = require("../preview")
local state = require("../state")

local Button = require("./Button")
local Label = require("./Label")

type Conflict = state.Conflict

local function ConflictRow(conflict: Conflict)
	Element("TextButton", {
		id = conflict.name,
		Text = "",
		AutoButtonColor = false,
		tags = { "auto-size-y", "flex-col", "gap-sm", "p-md", "corner-md" },
		BackgroundColor3 = if state.selected == conflict
			then Color3.fromRGB(48, 48, 56)
			else Color3.fromRGB(40, 40, 46),
		events = {
			Activated = function()
				state.selected = conflict
			end,
		},
	}, function()
		Label("path", conflict.path, state.statusColor(conflict))

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

		local oursSummary = format.sideSummary(conflict, "ours")
		local theirsSummary = format.sideSummary(conflict, "theirs")
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
				Button("view", "View", false, Color3.fromRGB(80, 80, 90), function()
					preview.focusOn(conflict)
				end)
			end
		end)
	end)
end

return ConflictRow
