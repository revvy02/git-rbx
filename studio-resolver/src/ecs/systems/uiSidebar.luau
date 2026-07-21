--[[
	Sidebar dock widget: explorer top half, properties bottom half, rendered
	immediate-mode every heartbeat.
]]

local packages = require("../../packages")
local Element = packages.miu.Element
local Portal = packages.miu.Portal

local theme = require("../../library/theme")

local types = require("../types")
local widgets = require("./library/widgets")

local ExplorerPanel = require("../../ui/ExplorerPanel")
local PropertyPanel = require("../../ui/PropertyPanel")

return function(_world: types.World, _cts: types.Cts, scheduler: types.Scheduler)
	local widget = widgets.create("RbxDiffMergeExplorer", "rbx-diff", Enum.InitialDockState.Left)

	scheduler:addSystem({
		name = "ui_sidebar",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("rbxdiff_ui_sidebar")
			Portal(widget, "rbx-diff-sidebar", function()
				theme.link()
				Element("Frame", {
					id = "sidebar",
					Size = UDim2.new(1, 0, 1, 0),
					tags = { "surface", "vs-border" },
				}, function()
					ExplorerPanel()
					PropertyPanel()
				end)
			end)
			debug.profileend()
		end,
	})
end
