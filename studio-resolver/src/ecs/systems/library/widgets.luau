--[[
	Dock widget creation. rodeo runs us at plugin identity but hands us no
	`plugin` global — the rodeo plugin itself is a Plugin instance under
	PluginDebugService, and CreateDockWidgetPluginGuiAsync works from there.
]]

local widgets = {}

function widgets.create(id: string, title: string, dockState: Enum.InitialDockState): DockWidgetPluginGui
	local rodeoPlugin: Plugin?
	for _, child in game:GetService("PluginDebugService"):GetChildren() do
		if child:IsA("Plugin") and child.Name:match("rodeo") then
			rodeoPlugin = child
			break
		end
	end

	local widget = (rodeoPlugin :: Plugin):CreateDockWidgetPluginGuiAsync(
		id,
		DockWidgetPluginGuiInfo.new(dockState, true, true, 300, 600, 240, 300)
	)
	widget.Title = title
	return widget
end

return widgets
