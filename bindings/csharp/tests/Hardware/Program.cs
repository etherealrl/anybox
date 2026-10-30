// a manual run through C# on a real box: it connects, prints what it found, moves the cursor right and back, clicks
// the left button where the cursor is, waits up to 20 s for you to press any button on the mouse behind the box,
// presses a B with Shift as a chord and a c as a chord tap, types " anybox types 1!" and Enter into whatever has
// focus, and closes. on a MAKCU it times a move at 50 percent interpolation and sets autorelease and turns it off
// again. on a Medius it also reads the polling rate and the render setting and reconnects through
// device.Medius(), and once anybox is closed it goes through medius' own ways in. pass a port to open that one,
// otherwise it takes the first box it finds. ANYBOX_LIB has to point at a release build of the capi
using System.Diagnostics;
using Anybox;
using MediusDevice = Anybox.Medius.Device;

try
{
    new Connect { Port = "NOPE99" }.Open();
}
catch (NotFoundException error)
{
    Console.WriteLine($"a port with nothing on it tried {string.Join(", ", error.Tried)}");
}
var connect = new Connect { Port = args.Length > 0 ? args[0] : null, Can = [BoxAction.Move] };
var port = "";
var onMedius = false;
using (var device = connect.Open())
{
    var info = device.Snapshot().Info;
    port = info.Port;
    Console.WriteLine($"{info.Family} on {info.Port}, model {info.Model}, firmware {info.Firmware}");
    using var events = device.Events(EventKinds.Buttons);
    for (var step = 0; step < 20; step++)
    {
        device.MoveBy(10, 0);
    }
    for (var step = 0; step < 20; step++)
    {
        device.MoveBy(-10, 0);
    }
    device.Click(Button.Left);
    using (var medius = device.Medius())
    {
        if (medius is not null)
        {
            onMedius = true;
            Console.WriteLine($"polling rate through medius: {medius.QueryRate()}");
            // the render setting is kept on the box, so it goes back exactly as it was
            var render = medius.QueryRender();
            medius.SetRender(render.Mode, render.Full);
            Console.WriteLine($"render through medius: {medius.QueryRender()}");
        }
    }
    Console.WriteLine("moved and clicked, now press a button on the mouse");
    var clock = Stopwatch.StartNew();
    var pressed = false;
    while (!pressed && clock.Elapsed < TimeSpan.FromSeconds(20))
    {
        if (events.Recv(TimeSpan.FromSeconds(20) - clock.Elapsed) is ButtonEvent { Pressed: true } button)
        {
            var late = (Monotonic.FromStopwatchTicks(Stopwatch.GetTimestamp()) - button.At.ReceivedNs) / 1000;
            Console.WriteLine($"got {button.Button} down, read {late} us ago on the monotonic clock");
            pressed = true;
        }
    }
    if (!pressed)
    {
        Console.WriteLine("no button came");
    }
    Console.WriteLine("click into a text field, typing in 3 s");
    Thread.Sleep(TimeSpan.FromSeconds(3));
    // chords first, a MAKCU refuses a key write for about 10 ms after a tap and typing waits for it to be done
    try
    {
        device.PressKeys(Key.LeftShift, Key.B);
        device.ReleaseKeys(Key.LeftShift, Key.B);
        device.TapKeys(Key.C);
        Console.WriteLine("pressed the chords");
    }
    catch (UnsupportedException error)
    {
        Console.WriteLine($"chords: {error.Message}");
    }
    try
    {
        device.TypeText(" anybox types 1!");
        device.TapKey(Key.Enter);
        Console.WriteLine("typed");
    }
    catch (UnsupportedException error)
    {
        Console.WriteLine($"typing: {error.Message}");
    }
    if (device.Makcu() is { } makcu)
    {
        var before = makcu.Interpolation();
        makcu.SetInterpolation(Interpolation.Percent(50));
        device.MoveBy(100, 0);
        var paying = Stopwatch.StartNew();
        while (makcu.Moving())
        {
        }
        Console.WriteLine($"interpolation was {before}, at 50 a move took {paying.ElapsedMilliseconds} ms");
        device.MoveBy(-100, 0);
        makcu.SetInterpolation(before);
        try
        {
            makcu.SetAutorelease(TimeSpan.FromMilliseconds(1500));
            Console.WriteLine($"autorelease set to {makcu.Autorelease()}");
            makcu.SetAutorelease(null);
        }
        catch (UnsupportedException error)
        {
            Console.WriteLine($"autorelease: {error.Message}");
        }
    }
    if (device.Ysc() is { } ysc)
    {
        Console.WriteLine($"YSC interpolation {ysc.Interpolation()}, serial {ysc.Serial()}");
    }
    if (device.Ferrum() is { } ferrum)
    {
        Console.WriteLine($"the mouse moved {ferrum.CatchXy(TimeSpan.FromMilliseconds(500))} in the last 500 ms");
    }
    Console.WriteLine($"box calls: MAKCU {device.Makcu() is not null}, YSC {device.Ysc() is not null}, Ferrum {device.Ferrum() is not null}");
    using (var medius = device.Medius())
    {
        if (medius is not null)
        {
            // the rescan finds this box again by its serial and MAC
            var before = medius.QueryVersion();
            medius.Reconnect();
            Console.WriteLine($"reconnect through medius: {before} before, {medius.QueryVersion()} after");
        }
    }
    device.Close();
}
Console.WriteLine("closed ok");
if (onMedius)
{
    ThroughMedius(port);
}

// the port is free again once anybox is closed. the finds take the first box that fits, which can be another one
static void ThroughMedius(string port)
{
    foreach (var each in MediusDevice.FindPorts())
    {
        Console.WriteLine($"medius sees port {each.Path}, serial {each.Serial ?? "none"}");
    }
    var id = "";
    foreach (var each in MediusDevice.List())
    {
        Console.WriteLine($"medius lists {each.Name} on {each.Port.Path}, id {each.Id}, serial {each.Serial ?? "none"}");
        if (each.Port.Path == port)
        {
            id = each.Id;
        }
    }
    using (var opened = MediusDevice.Open(port))
    {
        Console.WriteLine($"opened by port: {opened.QueryVersion()}");
    }
    using (var opened = MediusDevice.OpenById(id))
    {
        Console.WriteLine($"opened by id: {opened.QueryVersion()}");
    }
    using (var opened = MediusDevice.FindWhere(box => box.Port.Path == port))
    {
        Console.WriteLine($"opened by a match on the port: {opened.QueryVersion()}");
    }
    (string Name, Func<MediusDevice> Find)[] finds =
        [("Find", MediusDevice.Find), ("FindKeyboardBox", MediusDevice.FindKeyboardBox), ("FindMouseBox", MediusDevice.FindMouseBox)];
    foreach (var (name, find) in finds)
    {
        try
        {
            using var found = find();
            Console.WriteLine($"{name}: {found.QueryVersion()}");
        }
        catch (Anybox.Medius.MediusException error)
        {
            Console.WriteLine($"{name}: {error.Message}");
        }
    }
}
