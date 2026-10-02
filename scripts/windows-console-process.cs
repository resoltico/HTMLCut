using System;
using System.ComponentModel;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;

// Native test host only. The application retains its Rust unsafe-code prohibition.
public static class WindowsConsoleProcess
{
    [StructLayout(LayoutKind.Sequential)] struct Security { public int Length; public IntPtr Descriptor; public int Inherit; }
    [StructLayout(LayoutKind.Sequential)] struct Coord { public short X, Y; public Coord(short x, short y) { X = x; Y = y; } }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] struct Startup
    {
        public int Size; public string Reserved, Desktop, Title;
        public int X, Y, XSize, YSize, XChars, YChars, Fill, Flags;
        public short Show, ReservedSize; public IntPtr ReservedBytes, Input, Output, Error;
    }
    [StructLayout(LayoutKind.Sequential)] struct ProcessInfo { public IntPtr Process, Thread; public int ProcessId, ThreadId; }
    [StructLayout(LayoutKind.Explicit, CharSet = CharSet.Unicode, Size = 20)] struct InputRecord
    {
        [FieldOffset(0)] public short Type;
        [FieldOffset(4)] public int Down;
        [FieldOffset(8)] public short Repeat;
        [FieldOffset(10)] public short Key;
        [FieldOffset(14)] public char Character;
        [FieldOffset(16)] public int State;
    }
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool AllocConsole();
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool FreeConsole();
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool SetConsoleOutputCP(uint page);
    [DllImport("kernel32.dll")] static extern uint GetConsoleOutputCP();
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool SetConsoleMode(IntPtr handle, uint mode);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool GetConsoleMode(IntPtr handle, out uint mode);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool FlushConsoleInputBuffer(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true)] static extern IntPtr CreateConsoleScreenBuffer(uint access, uint share, ref Security security, uint flags, IntPtr reserved);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern IntPtr CreateFileW(string name, uint access, uint share, ref Security security, uint creation, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool CreatePipe(out IntPtr read, out IntPtr write, ref Security security, int size);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool SetHandleInformation(IntPtr handle, uint mask, uint flags);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern bool CreateProcessW(string application, StringBuilder arguments, IntPtr processSecurity, IntPtr threadSecurity, bool inherit, uint flags, IntPtr environment, string directory, ref Startup startup, out ProcessInfo process);
    [DllImport("kernel32.dll", SetLastError = true)] static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool GetExitCodeProcess(IntPtr handle, out uint code);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool TerminateProcess(IntPtr handle, uint code);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll")] static extern IntPtr GetCurrentProcess();
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool DuplicateHandle(IntPtr sourceProcess, IntPtr source, IntPtr targetProcess, out IntPtr duplicate, uint access, bool inherit, uint options);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool ReadFile(IntPtr handle, byte[] buffer, uint size, out uint read, IntPtr overlapped);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern bool ReadConsoleOutputCharacterW(IntPtr handle, StringBuilder buffer, uint size, Coord origin, out uint read);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern bool WriteConsoleInputW(IntPtr handle, InputRecord[] input, uint count, out uint written);

    public sealed class Result
    {
        public int ExitCode; public string StdoutBase64, StderrBase64, Screen;
        public bool ConsoleInput, ConsoleOutput; public int CodePage;
    }
    static void Check(bool success) { if (!success) throw new Win32Exception(Marshal.GetLastWin32Error()); }
    static void Close(ref IntPtr handle) { if (handle != IntPtr.Zero && handle != new IntPtr(-1)) CloseHandle(handle); handle = IntPtr.Zero; }
    static string Quote(string value)
    {
        // Every maintained argument is a simple token or fixture path, never shell syntax.
        if (value.IndexOf('"') >= 0 || value.EndsWith("\\")) throw new ArgumentException("Unsupported fixture argument");
        return "\"" + value + "\"";
    }
    static byte[] Drain(IntPtr handle)
    {
        using (var output = new MemoryStream())
        {
            var buffer = new byte[4096]; uint count;
            while (true)
            {
                if (!ReadFile(handle, buffer, (uint)buffer.Length, out count, IntPtr.Zero))
                {
                    int error = Marshal.GetLastWin32Error();
                    if (error != 109) throw new Win32Exception(error);
                    break;
                }
                if (count == 0) break;
                if (output.Length + count > 1024 * 1024) throw new IOException("Native stream evidence exceeded one MiB");
                output.Write(buffer, 0, (int)count);
            }
            return output.ToArray();
        }
    }
    public static void Begin()
    {
        // This host is a separate disposable test process, never the user's console.
        FreeConsole(); Check(AllocConsole()); Check(SetConsoleOutputCP(437));
    }
    public static void End() { Check(FreeConsole()); }

    public static Result Run(string binary, string[] arguments, string outputMode, bool consoleInput, string input, string readonlyFile)
    {
        Security security = new Security { Length = Marshal.SizeOf(typeof(Security)), Inherit = 1 };
        IntPtr outRead = IntPtr.Zero, outWrite = IntPtr.Zero, errRead = IntPtr.Zero, errWrite = IntPtr.Zero;
        IntPtr screen = IntPtr.Zero, source = IntPtr.Zero, readonlyHandle = IntPtr.Zero;
        ProcessInfo child = new ProcessInfo(); Task<byte[]> stdout = null, stderr = null;
        try
        {
            Check(CreatePipe(out outRead, out outWrite, ref security, 0));
            Check(CreatePipe(out errRead, out errWrite, ref security, 0));
            Check(SetHandleInformation(outRead, 1, 0)); Check(SetHandleInformation(errRead, 1, 0));
            screen = CreateConsoleScreenBuffer(0xC0000000, 3, ref security, 1, IntPtr.Zero);
            if (screen == new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error());
            uint mode; Check(GetConsoleMode(screen, out mode));
            source = CreateFileW(consoleInput ? "CONIN$" : "NUL", consoleInput ? 0xC0000000 : 0x80000000, 3, ref security, 3, 0, IntPtr.Zero);
            if (source == new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error());
            if (consoleInput) { Check(GetConsoleMode(source, out mode)); Check(SetConsoleMode(source, 3)); Check(FlushConsoleInputBuffer(source)); }
            IntPtr output = outWrite, error = errWrite;
            if (outputMode == "stdout-console") output = screen;
            else if (outputMode == "stderr-console") error = screen;
            else if (outputMode == "readonly-console")
            {
                Check(DuplicateHandle(GetCurrentProcess(), screen, GetCurrentProcess(), out readonlyHandle, 0x80000000, true, 0));
                Check(GetConsoleMode(readonlyHandle, out mode));
                output = readonlyHandle;
            }
            else if (outputMode == "readonly")
            {
                readonlyHandle = CreateFileW(readonlyFile, 0x80000000, 3, ref security, 3, 0, IntPtr.Zero);
                if (readonlyHandle == new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error());
                output = readonlyHandle;
            }
            else if (outputMode == "invalid") output = new IntPtr(-1);
            else throw new ArgumentException("Unknown native output fixture mode");
            Startup startup = new Startup { Size = Marshal.SizeOf(typeof(Startup)), Flags = 0x100, Input = source, Output = output, Error = error };
            var command = new StringBuilder(Quote(binary));
            foreach (string argument in arguments) command.Append(" ").Append(Quote(argument));
            Check(CreateProcessW(binary, command, IntPtr.Zero, IntPtr.Zero, true, 0, IntPtr.Zero, null, ref startup, out child));
            Close(ref outWrite); Close(ref errWrite);
            IntPtr stdoutHandle = outRead, stderrHandle = errRead;
            stdout = Task.Run(() => Drain(stdoutHandle)); stderr = Task.Run(() => Drain(stderrHandle));
            if (consoleInput)
            {
                var keys = new InputRecord[input.Length];
                for (int i = 0; i < input.Length; i++)
                    keys[i] = new InputRecord { Type = 1, Down = 1, Repeat = 1, Character = input[i], Key = input[i] == '\r' ? (short)13 : input[i] == '\u001a' ? (short)90 : (short)0, State = input[i] == '\u001a' ? 8 : 0 };
                uint written; Check(WriteConsoleInputW(source, keys, (uint)keys.Length, out written));
                if (written != keys.Length) throw new IOException("Incomplete console input fixture");
            }
            if (WaitForSingleObject(child.Process, 10000) != 0)
            { TerminateProcess(child.Process, 124); WaitForSingleObject(child.Process, 5000); throw new TimeoutException("Native candidate process did not finish"); }
            uint exit; Check(GetExitCodeProcess(child.Process, out exit));
            if (!Task.WaitAll(new Task[] { stdout, stderr }, 5000)) throw new TimeoutException("Native pipe drain did not finish");
            var text = new StringBuilder(4096); uint characters;
            Check(ReadConsoleOutputCharacterW(screen, text, 4096, new Coord(0, 0), out characters));
            return new Result { ExitCode = (int)exit, StdoutBase64 = Convert.ToBase64String(stdout.Result),
                StderrBase64 = Convert.ToBase64String(stderr.Result), Screen = text.ToString().TrimEnd(' ', '\0'),
                ConsoleInput = consoleInput, ConsoleOutput = outputMode.EndsWith("console"), CodePage = (int)GetConsoleOutputCP() };
        }
        finally
        {
            if (child.Process != IntPtr.Zero && WaitForSingleObject(child.Process, 0) != 0) { TerminateProcess(child.Process, 124); WaitForSingleObject(child.Process, 5000); }
            Close(ref child.Thread); Close(ref child.Process); Close(ref outWrite); Close(ref errWrite);
            Close(ref outRead); Close(ref errRead); Close(ref readonlyHandle); Close(ref source); Close(ref screen);
        }
    }
}
