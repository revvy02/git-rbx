--[[
	Studio's own explorer icons (spritesheet slices), cached per class — the
	same iconography as the native Explorer, icon packs respected.
]]

local StudioService = game:GetService("StudioService")

export type ClassIcon = { Image: string, ImageRectOffset: Vector2, ImageRectSize: Vector2 }

local cache: { [string]: ClassIcon } = {}

local function classIcon(className: string): ClassIcon
	local icon = cache[className]
	if not icon then
		icon = StudioService:GetClassIcon(className) :: ClassIcon
		cache[className] = icon
	end
	return icon
end

return classIcon
