"""a manual run through Python on a real box: it connects, prints what it found, moves the cursor right and back,
clicks the left button where the cursor is, waits up to 20 s for you to press any button on the mouse behind the
box, presses a B with Shift as a chord and a c as a chord tap, types " anybox types 1!" and Enter into whatever has
focus, and closes. on a Medius it also reads the polling rate through device.medius(), on a MAKCU it times a move
at 50 percent interpolation and sets autorelease and turns it off again. pass a port to open that one, otherwise
it takes the first box it finds"""

import sys
import time

import anybox

try:
    anybox.Connect(port="NOPE99").open()
except anybox.NotFoundError as error:
    print(f"a port with nothing on it tried {error.tried}")
port = sys.argv[1] if len(sys.argv) > 1 else None
with anybox.Connect(port=port, can=[anybox.Action.MOVE]).open() as device:
    info = device.snapshot().info
    print(f"{info.family.name} on {info.port}, model {info.model}, firmware {info.firmware}")
    with device.events(anybox.EventKinds.BUTTONS) as events:
        for _ in range(20):
            device.move_by(10, 0)
        for _ in range(20):
            device.move_by(-10, 0)
        device.click(anybox.Button.LEFT)
        if (medius := device.medius()) is not None:
            print(f"polling rate through medius: {medius.query_rate()}")
            medius.close()
        print("moved and clicked, now press a button on the mouse")
        deadline = time.monotonic() + 20
        while (left := deadline - time.monotonic()) > 0:
            event = events.recv(left)
            if isinstance(event, anybox.ButtonEvent) and event.pressed:
                late = (time.perf_counter_ns() - event.at.received_ns) // 1000
                print(f"got {event.button.name} down, read {late} us ago on the monotonic clock")
                break
        else:
            print("no button came")
        print("click into a text field, typing in 3 s")
        time.sleep(3)
        # chords first, a MAKCU refuses a key write for about 10 ms after a tap and typing waits for it to be done
        try:
            device.press_keys(anybox.Key.LEFT_SHIFT, anybox.Key.B)
            device.release_keys(anybox.Key.LEFT_SHIFT, anybox.Key.B)
            device.tap_keys(anybox.Key.C)
            print("pressed the chords")
        except anybox.UnsupportedError as error:
            print(f"chords: {error}")
        try:
            device.type_text(" anybox types 1!")
            device.tap_key(anybox.Key.ENTER)
            print("typed")
        except anybox.UnsupportedError as error:
            print(f"typing: {error}")
    if (makcu := device.makcu()) is not None:
        before = makcu.interpolation()
        makcu.set_interpolation(anybox.Interpolation.percent(50))
        device.move_by(100, 0)
        started = time.perf_counter()
        while makcu.moving():
            pass
        print(f"interpolation was {before}, at 50 a move took {(time.perf_counter() - started) * 1000:.0f} ms")
        device.move_by(-100, 0)
        makcu.set_interpolation(before)
        try:
            makcu.set_autorelease(1.5)
            print(f"autorelease set to {makcu.autorelease()} s")
            makcu.set_autorelease(None)
        except anybox.UnsupportedError as error:
            print(f"autorelease: {error}")
    if (ysc := device.ysc()) is not None:
        print(f"YSC interpolation {ysc.interpolation()}, serial {ysc.serial()}")
    if (ferrum := device.ferrum()) is not None:
        print(f"the mouse moved {ferrum.catch_xy(0.5)} in the last 500 ms")
    print(f"box calls: MAKCU {makcu is not None}, YSC {ysc is not None}, Ferrum {ferrum is not None}")
print("closed ok")
