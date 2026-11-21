// Print a process's first window title, origin and size as tab-separated
// values: title <TAB> x <TAB> y <TAB> width <TAB> height
//
// Usage: window <pid>
//        window <pid> move <x> <y>
//        window <pid> resize <width> <height>
//        window <pid> layers
//        window <pid> id [layer]
//        window front
//
// The GUI smoke test uses the live origin because window managers move
// windows between launches. `move` brings the window on screen and to the
// front, which keeps absolute mouse coordinates stable across runs.
//
// `layers` lists every on-screen window for the process as
// window <TAB> layer <TAB> x <TAB> y <TAB> width <TAB> height,
// so a probe can confirm the capture panel floats above normal windows.
// `id` prints the CGWindowID of the process's largest window (or the largest
// at a given layer), for `screencapture -l` and clean screenshots.
// `front` prints the frontmost application's process id.

import AppKit
import ApplicationServices
import Foundation

if CommandLine.arguments.count >= 2, CommandLine.arguments[1] == "front" {
    if let front = NSWorkspace.shared.frontmostApplication {
        print(front.processIdentifier)
    }
    exit(0)
}

guard CommandLine.arguments.count > 1, let pid = pid_t(CommandLine.arguments[1]) else {
    exit(1)
}

if CommandLine.arguments.count >= 3, CommandLine.arguments[2] == "id" {
    let layerFilter = CommandLine.arguments.count >= 4 ? Int(CommandLine.arguments[3]) : nil
    let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
    let windows = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] ?? []
    var best: (number: Int, area: Double)?
    for window in windows {
        guard let owner = window[kCGWindowOwnerPID as String] as? Int,
            owner == Int(pid)
        else { continue }
        let layer = window[kCGWindowLayer as String] as? Int ?? 0
        if let layerFilter, layer != layerFilter { continue }
        let bounds = window[kCGWindowBounds as String] as? [String: Any] ?? [:]
        let width = bounds["Width"] as? Double ?? 0
        let height = bounds["Height"] as? Double ?? 0
        let area = width * height
        if best == nil || area > best!.area {
            best = (window[kCGWindowNumber as String] as? Int ?? 0, area)
        }
    }
    if let best { print(best.number) }
    exit(0)
}

if CommandLine.arguments.count >= 3, CommandLine.arguments[2] == "layers" {
    let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
    let windows = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] ?? []
    for window in windows {
        guard let owner = window[kCGWindowOwnerPID as String] as? Int,
            owner == Int(pid)
        else { continue }
        let number = window[kCGWindowNumber as String] as? Int ?? 0
        let layer = window[kCGWindowLayer as String] as? Int ?? 0
        let bounds = window[kCGWindowBounds as String] as? [String: Any] ?? [:]
        let x = bounds["X"] as? Double ?? 0
        let y = bounds["Y"] as? Double ?? 0
        let width = bounds["Width"] as? Double ?? 0
        let height = bounds["Height"] as? Double ?? 0
        print("\(number)\t\(layer)\t\(x)\t\(y)\t\(width)\t\(height)")
    }
    exit(0)
}
let app = AXUIElementCreateApplication(pid)
var value: CFTypeRef?
AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value)
guard let windows = value as? [AXUIElement], let window = windows.first else {
    exit(1)
}

if CommandLine.arguments.count >= 5, CommandLine.arguments[2] == "resize",
    let width = Double(CommandLine.arguments[3]), let height = Double(CommandLine.arguments[4])
{
    var size = CGSize(width: width, height: height)
    if let resized = AXValueCreate(.cgSize, &size) {
        AXUIElementSetAttributeValue(window, kAXSizeAttribute as CFString, resized)
    }
    usleep(300_000)
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
