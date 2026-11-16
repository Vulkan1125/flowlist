// Print a process's first window title, origin and size as tab-separated
// values: title <TAB> x <TAB> y <TAB> width <TAB> height
//
// Usage: window <pid>
//        window <pid> move <x> <y>
//
// The GUI smoke test uses the live origin because window managers move
// windows between launches. `move` brings the window on screen and to the
// front, which keeps absolute mouse coordinates stable across runs.

import AppKit
import ApplicationServices
import Foundation

guard CommandLine.arguments.count > 1, let pid = pid_t(CommandLine.arguments[1]) else {
    exit(1)
}
let app = AXUIElementCreateApplication(pid)
var value: CFTypeRef?
AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value)
guard let windows = value as? [AXUIElement], let window = windows.first else {
    exit(1)
}

if CommandLine.arguments.count >= 5, CommandLine.arguments[2] == "move",
    let x = Double(CommandLine.arguments[3]), let y = Double(CommandLine.arguments[4])
{
    var position = CGPoint(x: x, y: y)
    if let moved = AXValueCreate(.cgPoint, &position) {
        AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, moved)
    }
    NSRunningApplication(processIdentifier: pid)?.activate(options: [.activateAllWindows])
    usleep(300_000)
}

var titleValue: CFTypeRef?
AXUIElementCopyAttributeValue(window, kAXTitleAttribute as CFString, &titleValue)
let title = titleValue as? String ?? ""

var positionValue: CFTypeRef?
AXUIElementCopyAttributeValue(window, kAXPositionAttribute as CFString, &positionValue)
var sizeValue: CFTypeRef?
AXUIElementCopyAttributeValue(window, kAXSizeAttribute as CFString, &sizeValue)

var position = CGPoint.zero
var size = CGSize.zero
if let value = positionValue, CFGetTypeID(value) == AXValueGetTypeID() {
    AXValueGetValue(value as! AXValue, .cgPoint, &position)
}
if let value = sizeValue, CFGetTypeID(value) == AXValueGetTypeID() {
    AXValueGetValue(value as! AXValue, .cgSize, &size)
}

print("\(title)\t\(position.x)\t\(position.y)\t\(size.width)\t\(size.height)")
