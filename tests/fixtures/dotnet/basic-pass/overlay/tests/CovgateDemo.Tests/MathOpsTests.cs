using CovgateDemo;
using Xunit;

namespace CovgateDemo.Tests;

public class MathOpsTests
{
    [Fact]
    public void Add_positive_numbers_returns_sum()
    {
        Assert.Equal(3, MathOps.Add(1, 2));
    }

    [Fact]
    public void Add_negative_a_returns_b()
    {
        Assert.Equal(2, MathOps.Add(-1, 2));
    }
}
