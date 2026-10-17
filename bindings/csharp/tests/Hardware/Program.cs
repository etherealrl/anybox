// a manual run through C# on a real box: it connects, prints what it found, moves the cursor right and back, clicks
// the left button where the cursor is, then waits up to 20 s for you to press any button on the mouse behind the box
// and closes. on a Medius it also reads the polling rate and the render setting and reconnects through
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
