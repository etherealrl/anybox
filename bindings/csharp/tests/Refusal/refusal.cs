// what a program gets when the library can't be used, run on its own by the tests because the library only gets
// looked up once per process. with `default-filter` it's the other way round: a default Medius filter used first
// thing still finds the library
#:project ../../src/Anybox/Anybox.csproj

try
{
    if (args is ["default-filter"])
    {
        Console.WriteLine(default(Anybox.Medius.CatchFilter).WithCapture(8).Capture);
        return 0;
    }
    Console.WriteLine(Anybox.Library.Version);
    return 0;
}
catch (Anybox.AnyboxException error) when (error.Status is null)
{
    Console.WriteLine(error.Message);
    return 1;
}
