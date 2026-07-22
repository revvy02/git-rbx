-- Selected-instance properties dock widget, synchronized through the ECS world.

local packages = require("../../packages")
local Element = packages.miu.Element
local Portal = packages.miu.Portal

local theme = require("../../library/theme")

local types = require("../types")
local widgets = require("./library/widgets")

local PropertyPanel = require("../../ui/PropertyPanel")

return function(_world: types.World, _cts: types.Cts, scheduler: types.Scheduler)
	local widget = widgets.create("RbxDiffMergeProperties", "rbx-diff — properties", Enum.InitialDockState.Left, {
		width = 340,
		height = 500,
		minWidth = 240,
		minHeight = 180,
	})

	scheduler:addSystem({
		name = "ui_properties",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("rbxdiff_ui_properties")
			Portal(widget, "rbx-diff-properties", function()
				theme.link()
				Element("Frame", {
					id = "properties-widget",
					Size = UDim2.fromScale(1, 1),
					tags = { "surface", "vs-border" },
				}, function()
					PropertyPanel()
				end)
			end)
			debug.profileend()
		end,
	})
end
