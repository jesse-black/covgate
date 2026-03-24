using CovgateDemo;

namespace CovgateDemo.Tests;

public class AsyncHandlerTests
{
    [Fact]
    public async Task HandlesWithoutProjectionInBase()
    {
        Assert.Equal(new[] { "a", "b" }, await AsyncHandler.HandleAsync(new[] { "a", "b" }));
    }
}
