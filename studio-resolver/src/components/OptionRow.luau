local Element = require("../miu/Instance")

-- One row of the comparison card: side name + formatted value, tinted when
-- staged, click to stage.
local function OptionRow(id: string, name: string, value: string?, color: Color3, staged: boolean, onClick: () -> ())
	Element("TextButton", {
		id = id,
		Text = "",
		Size = UDim2.new(1, 0, 0, 0),
		AutomaticSize = Enum.AutomaticSize.Y,
		tags = { "corner-sm" },
		BackgroundColor3 = if staged then color else Color3.fromRGB(52, 52, 58),
		BackgroundTransparency = if staged then 0.6 else 0,
		AutoButtonColor = true,
		events = { Activated = onClick },
	}, function()
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 6),
			PaddingRight = UDim.new(0, 6),
			PaddingTop = UDim.new(0, 3),
			PaddingBottom = UDim.new(0, 3),
		})
		Element("UIListLayout", {
			id = "layout",
			FillDirection = Enum.FillDirection.Horizontal,
			Padding = UDim.new(0, 8),
			VerticalAlignment = Enum.VerticalAlignment.Center,
		})
		Element("TextLabel", {
			id = "name",
			Text = name,
			Size = UDim2.new(0, 44, 0, 16),
			BackgroundTransparency = 1,
			TextColor3 = color,
			TextXAlignment = Enum.TextXAlignment.Left,
			TextSize = 12,
			Font = Enum.Font.GothamMedium,
		})
		Element("TextLabel", {
			id = "value",
			RichText = true,
			Text = value or "?",
			tags = { "text-base", "auto-size-y" },
			TextColor3 = Color3.fromRGB(225, 225, 230),
			TextXAlignment = Enum.TextXAlignment.Left,
			TextSize = 12,
			Font = Enum.Font.RobotoMono,
			TextWrapped = true,
		})
	end)
end

return OptionRow
