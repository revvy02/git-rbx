--[[
	One cell of a change list, after Rojo's DisplayValue: colors get a swatch
	plus `r, g, b`, strings are quoted, everything else is the typed value's
	text form. Accepts either Rust's serialized PropertyValue or a RichText
	override.
]]

local Element = require("@shared/packages").miu.Element

local format = require("@shared/library/format")
local theme = require("@shared/library/theme")

local function colorOf(value: any): Color3?
	if type(value) == "table" and value.type == "color3" and value.value then
		local v = value.value
		return Color3.new(v.r, v.g, v.b)
	end
	return nil
end

local function DisplayValue(id: string, value: any, text: string?, color: Color3)
	local swatch = if text == nil then colorOf(value) else nil
	local label = text
		or (
			if swatch
				then string.format("%d, %d, %d", swatch.R * 255, swatch.G * 255, swatch.B * 255)
				else format.patchValue(value)
		)

	Element("Frame", {
		id = id,
		Size = UDim2.fromScale(1, 1),
		BackgroundTransparency = 1,
	}, function()
		if swatch then
			Element("Frame", {
				id = "swatch",
				Size = UDim2.fromOffset(16, 16),
				Position = UDim2.fromScale(0, 0.5),
				AnchorPoint = Vector2.new(0, 0.5),
				BackgroundColor3 = swatch,
				BackgroundTransparency = 0,
				BorderSizePixel = 0,
			}, function()
				Element("UICorner", { id = "corner", CornerRadius = UDim.new(0, 4) })
				Element("UIStroke", { id = "stroke", Color = theme.border, Thickness = 1 })
			end)
		end
		Element("TextLabel", {
			id = "label",
			RichText = true,
			Text = label,
			Position = UDim2.fromOffset(if swatch then 21 else 0, 0),
			Size = UDim2.new(1, if swatch then -21 else 0, 1, 0),
			tags = { "text", "text-small" },
			TextColor3 = color,
			TextTruncate = Enum.TextTruncate.AtEnd,
		})
	end)
end

return DisplayValue
