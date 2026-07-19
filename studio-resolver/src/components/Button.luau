local Element = require("../miu/Instance")

local function Button(id: string, text: string, highlighted: boolean, color: Color3, onClick: () -> ())
	Element("TextButton", {
		id = id,
		Text = text,
		Size = UDim2.new(0, 64, 0, 22),
		tags = { "corner-sm" },
		BackgroundColor3 = if highlighted then color else Color3.fromRGB(60, 60, 65),
		TextColor3 = Color3.fromRGB(240, 240, 240),
		TextSize = 13,
		Font = Enum.Font.GothamMedium,
		AutoButtonColor = true,
		events = { Activated = onClick },
	})
end

return Button
