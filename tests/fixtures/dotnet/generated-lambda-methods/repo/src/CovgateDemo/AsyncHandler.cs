namespace CovgateDemo;

public static class AsyncHandler
{
    public static async Task<string[]> HandleAsync(string[] values)
    {
        await Task.Yield();
        return values.ToArray();
    }
}
