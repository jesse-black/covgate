using CovgateDemo;

namespace CovgateDemo.Tests;

public class AsyncHandlerTests
{
    [Fact]
    public async Task HandlesWithGeneratedLambdaProjection()
    {
        Assert.Equal(new[] { "A", "B" }, await AsyncHandler.HandleAsync(new[] { "a", "b" }));
    }
}
