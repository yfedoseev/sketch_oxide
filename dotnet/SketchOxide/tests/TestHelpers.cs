using System.Text;

namespace SketchOxide.Tests
{
    public static class TestHelpers
    {
        /// <summary>
        /// Converts a string to UTF-8 bytes for testing purposes.
        /// </summary>
        public static byte[] ToBytes(this string value)
        {
            return Encoding.UTF8.GetBytes(value);
        }

        /// <summary>
        /// Converts a string to UTF-8 bytes using Encoding directly.
        /// </summary>
        public static byte[] GetBytes(this string value)
        {
            return Encoding.UTF8.GetBytes(value);
        }
    }
}
