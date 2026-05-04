namespace CovgateDemo;

public static class MathOps
{
    public static int Add(int a, int b)
    {
        if (a < 0)
        {
            return b;
        }

        return a + b;
    }

    public static int Subtract(int a, int b)
    {
        return a - b;
    }
}
