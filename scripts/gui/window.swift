// Print a process's first window title, origin and size as tab-separated
// values: title <TAB> x <TAB> y <TAB> width <TAB> height
//
// Usage: window <pid>
//
// The GUI smoke test uses the live origin because window managers move
// windows between launches.

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
