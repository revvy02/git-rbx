-- @rodeo exec --return class_icons.json
local StudioService = game:GetService("StudioService")

local classIcons = {}
local classes = game:GetService("ReflectionService"):GetClasses()

print(string.format("[class_icons] Found %d classes to process", #classes))

local successCount = 0
local failCount = 0

for i, class in ipairs(classes) do
	local success, iconData = pcall(StudioService.GetClassIcon, StudioService, class.Name)
	if success and iconData then
		classIcons[class.Name] = iconData
		successCount = successCount + 1
	else
		failCount = failCount + 1
		if not success then
			print(string.format("[class_icons] Failed to get icon for %s: %s", class.Name, tostring(iconData)))
		end
	end

	-- Progress log every 100 classes
	if i % 100 == 0 then
		print(string.format("[class_icons] Progress: %d/%d classes processed", i, #classes))
	end
end

print(string.format("[class_icons] Complete: %d success, %d failed, %d total", successCount, failCount, #classes))

return classIcons -- rodeo handles JSON encoding
