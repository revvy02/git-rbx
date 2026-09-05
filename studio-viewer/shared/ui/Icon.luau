local Element = require("@shared/packages").miu.Element

local iconify = require("@shared/generated/iconify")
local images = require("@shared/generated/images")

local function Icon(id: string, name: string, color: Color3, size: UDim2?)
	local data = iconify(name, "ui")
	Element("ImageLabel", {
		id = id,
		Image = images.iconify.ui[tostring(data.sheet)],
		ImageRectOffset = data.offset,
		ImageRectSize = data.size,
		ImageColor3 = color,
		Size = size or UDim2.fromOffset(data.size.X, data.size.Y),
		AnchorPoint = Vector2.new(0.5, 0.5),
		Position = UDim2.fromScale(0.5, 0.5),
		BackgroundTransparency = 1,
	})
end

return Icon
