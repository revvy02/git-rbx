local Element = require("../miu/Instance")

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

return Label
