using CovgateDemo;
using Xunit;

namespace CovgateDemo.Tests;

public class MathOpsTests
{
    [Fact]
    public void Add_positive_numbers_returns_incremented_value()
    {
        Assert.Equal(2, MathOps.Add(1));
    }
}
