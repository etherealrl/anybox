"""a manual run through Python on a real box: it connects, prints what it found, moves the cursor right and back,
clicks the left button where the cursor is, then waits up to 20 s for you to press any button on the mouse behind
the box and closes. on a Medius it also reads the polling rate through device.medius(). pass a port to open that
one, otherwise it takes the first box it finds"""

import sys
import time

import anybox

connect = anybox.Connect(port=sys.argv[1]) if len(sys.argv) > 1 else anybox.Connect()
with connect.open() as device:
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
                print(f"got {event.button.name} down")
                break
        else:
            print("no button came")
print("closed ok")
