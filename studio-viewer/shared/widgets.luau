--[[
	Dock widget creation. rodeo runs us at plugin identity but hands us no
	`plugin` global — the rodeo plugin itself is a Plugin instance under
	PluginDebugService, and CreateDockWidgetPluginGuiAsync works from there.
]]

local widgets = {}

export type Size = {
	width: number?,
	height: number?,
	minWidth: number?,
	minHeight: number?,
}

function widgets.create(id: string, title: string, dockState: Enum.InitialDockState, size: Size?): DockWidgetPluginGui
	local rodeoPlugin: Plugin?
	for _, child in game:GetService("PluginDebugService"):GetChildren() do
		if child:IsA("Plugin") and child.Name:match("rodeo") then
			rodeoPlugin = child
			break
		end
	end

	local widget = (rodeoPlugin :: Plugin):CreateDockWidgetPluginGuiAsync(
		id,
		DockWidgetPluginGuiInfo.new(
			dockState,
			true,
			true,
			if size and size.width then size.width else 300,
			if size and size.height then size.height else 600,
			if size and size.minWidth then size.minWidth else 240,
			if size and size.minHeight then size.minHeight else 200
		)
	)
	widget.Title = title
	return widget
end

return widgets
