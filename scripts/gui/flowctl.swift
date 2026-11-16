// Mouse and keyboard events for the GUI smoke test.
//
// Usage: flowctl mouse <x> <y>
//        flowctl move <x> <y>
//        flowctl drag <x1> <y1> <x2> <y2>
//        flowctl rightmouse <x> <y>
//        flowctl key <keycode> [cmd|shift|ctrl|cmdshift|ctrlshift]
//        flowctl text <string>
//
// This is a test helper, not part of the app. It needs Accessibility
// permission for the terminal that runs it.

import AppKit
import CoreGraphics

func mouse(_ x: Double, _ y: Double) {
    let point = CGPoint(x: x, y: y)
    for (type, delay) in [
        (CGEventType.mouseMoved, 60_000),
        (CGEventType.leftMouseDown, 40_000),
        (CGEventType.leftMouseUp, 0),
    ] {
        let event = CGEvent(
            mouseEventSource: nil,
            mouseType: type,
            mouseCursorPosition: point,
            mouseButton: .left
        )
        event?.flags = []
        event?.post(tap: .cghidEventTap)
        usleep(useconds_t(delay))
    }
}

func rightMouse(_ x: Double, _ y: Double) {
    let point = CGPoint(x: x, y: y)
    for (type, delay) in [
        (CGEventType.mouseMoved, 60_000),
        (CGEventType.rightMouseDown, 40_000),
        (CGEventType.rightMouseUp, 0),
    ] {
        let event = CGEvent(
            mouseEventSource: nil,
            mouseType: type,
            mouseCursorPosition: point,
            mouseButton: .right
        )
        event?.flags = []
        event?.post(tap: .cghidEventTap)
        usleep(useconds_t(delay))
    }
}

func move(_ x: Double, _ y: Double) {    let event = CGEvent(
        mouseEventSource: nil,
        mouseType: .mouseMoved,
        mouseCursorPosition: CGPoint(x: x, y: y),
        mouseButton: .left
    )
    event?.flags = []
    event?.post(tap: .cghidEventTap)
}

func flags(_ name: String?) -> CGEventFlags {
    guard let name else { return [] }
    switch name {
    case "cmd": return .maskCommand
    case "shift": return .maskShift
    case "ctrl": return .maskControl
    case "cmdshift": return [.maskCommand, .maskShift]
    case "ctrlshift": return [.maskControl, .maskShift]
    case "altcmd": return [.maskCommand, .maskAlternate]
    case "ctrlcmd": return [.maskCommand, .maskControl]
    case "ctrlalt": return [.maskControl, .maskAlternate]
    default: return []
    }
}

func key(_ code: CGKeyCode, _ modifier: String?) {
    let pressed = flags(modifier)
    let down = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true)
    down?.flags = pressed
    down?.post(tap: .cghidEventTap)
    usleep(40_000)
    let up = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false)
    up?.flags = pressed
    up?.post(tap: .cghidEventTap)
}

func type(_ text: String) {
    for scalar in text.unicodeScalars {
        var unit = UniChar(scalar.value & 0xFFFF)
        let down = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true)
        down?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &unit)
        down?.post(tap: .cghidEventTap)
        let up = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false)
        up?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &unit)
        up?.post(tap: .cghidEventTap)
        usleep(12_000)
    }
}

func drag(_ x1: Double, _ y1: Double, _ x2: Double, _ y2: Double) {
    let start = CGPoint(x: x1, y: y1)
    let down = CGEvent(
        mouseEventSource: nil,
        mouseType: .leftMouseDown,
        mouseCursorPosition: start,
        mouseButton: .left
    )
    down?.flags = []
    down?.post(tap: .cghidEventTap)
    usleep(60_000)
    for step in 1...12 {
        let fraction = Double(step) / 12.0
        let point = CGPoint(
            x: x1 + (x2 - x1) * fraction,
            y: y1 + (y2 - y1) * fraction
        )
        let move = CGEvent(
            mouseEventSource: nil,
            mouseType: .leftMouseDragged,
            mouseCursorPosition: point,
            mouseButton: .left
        )
        move?.flags = []
        move?.post(tap: .cghidEventTap)
        usleep(15_000)
    }
    let up = CGEvent(
        mouseEventSource: nil,
        mouseType: .leftMouseUp,
        mouseCursorPosition: CGPoint(x: x2, y: y2),
        mouseButton: .left
    )
    up?.flags = []
    up?.post(tap: .cghidEventTap)
}

let args = CommandLine.arguments
guard args.count >= 2 else { exit(1) }
switch args[1] {
case "mouse": mouse(Double(args[2])!, Double(args[3])!)
case "drag":
    drag(
        Double(args[2])!,
        Double(args[3])!,
        Double(args[4])!,
        Double(args[5])!
    )
case "rightmouse": rightMouse(Double(args[2])!, Double(args[3])!)
case "move": move(Double(args[2])!, Double(args[3])!)
case "key": key(CGKeyCode(args[2])!, args.count > 3 ? args[3] : nil)
case "text": type(args[2])
case "pos": print(NSEvent.mouseLocation)
default: break
}
