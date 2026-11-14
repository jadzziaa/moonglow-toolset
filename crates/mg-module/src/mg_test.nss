// mg_test: assertions for `mg test` (Moonglow Toolset). Marker format 2.
//
// A test is a script named test_*, with a main(), that includes this file
// and calls the functions below. `mg test MODULE` runs every one of them in
// the game's server, each as an event of its own, and reads what they wrote
// to the server's log:
//
//     MG_TEST PASS <name>
//     MG_TEST FAIL <name> :: <detail>
//
// An assertion made after its test's main() has returned (in a
// DelayCommand, an AssignCommand) names its test first, the script that
// queued it, which the server still runs it as (format 2):
//
//     MG_TEST FAIL [test_name] <name> :: <detail>
//
// A module that runs its tests itself (`mg test --driver none`) calls
// TestsDone() when the last one is through; its lines name no test.

const int MG_TEST_FORMAT = 2;

// The name the following assertions of this test are grouped under ("" for
// none), its delayed ones too.
void Describe(string sGroup);

// One passed, or failed, test case by name.
void Pass(string sName);
void Fail(string sName, string sDetail = "");

// Passes if bCondition, else fails with sDetail. Returns bCondition.
int Assert(int bCondition, string sName, string sDetail = "");

// Pass if the two are equal; a failure says what was got and expected.
int AssertEqualInt(int nActual, int nExpected, string sName);
int AssertEqualString(string sActual, string sExpected, string sName);
int AssertEqualFloat(float fActual, float fExpected, string sName, float fWithin = 0.0001);

// Passes if oObject is a valid object.
int AssertValid(object oObject, string sName);

// All tests are through: `mg test` stops the server. Only for a module
// that runs its tests itself; `mg test`'s own driver says it.
void TestsDone();

// The test making an assertion: the first script the server is running
// that is not `mg test`'s driver. In a delayed command that is the script
// that queued it.
string MgTestScript()
{
    string sDriver = GetLocalString(GetModule(), "MG_TEST_DRIVER");
    int nLevels = GetScriptRecursionLevel();
    int i;
    for (i = 0; i <= nLevels; i++)
    {
        string sScript = GetScriptName(i);
        if (sScript != sDriver) return sScript;
    }
    return "";
}

string MgTestName(string sName)
{
    object oModule = GetModule();
    string sScript = MgTestScript();
    string sGroup = GetLocalString(oModule, "MG_TEST_GROUP");
    if (sScript != "") sGroup = GetLocalString(oModule, "MG_TEST_GROUP " + sScript);
    if (sGroup != "") sName = sGroup + ": " + sName;
    // (Made outside the driver's run of its test: named by its test.)
    string sDriver = GetLocalString(oModule, "MG_TEST_DRIVER");
    if (sDriver != "" && sScript != "" && GetScriptName(0) != sDriver)
        sName = "[" + sScript + "] " + sName;
    return sName;
}

void Describe(string sGroup)
{
    object oModule = GetModule();
    string sScript = MgTestScript();
    if (sScript == "") SetLocalString(oModule, "MG_TEST_GROUP", sGroup);
    else SetLocalString(oModule, "MG_TEST_GROUP " + sScript, sGroup);
}

void Pass(string sName)
{
    WriteTimestampedLogEntry("MG_TEST PASS " + MgTestName(sName));
}

void Fail(string sName, string sDetail = "")
{
    if (sDetail == "")
        WriteTimestampedLogEntry("MG_TEST FAIL " + MgTestName(sName));
    else
        WriteTimestampedLogEntry("MG_TEST FAIL " + MgTestName(sName) + " :: " + sDetail);
}

int Assert(int bCondition, string sName, string sDetail = "")
{
    if (bCondition) Pass(sName);
    else Fail(sName, sDetail);
    return bCondition;
}

int AssertEqualInt(int nActual, int nExpected, string sName)
{
    return Assert(nActual == nExpected, sName,
        "got " + IntToString(nActual) + ", expected " + IntToString(nExpected));
}

int AssertEqualString(string sActual, string sExpected, string sName)
{
    return Assert(sActual == sExpected, sName,
        "got \"" + sActual + "\", expected \"" + sExpected + "\"");
}

int AssertEqualFloat(float fActual, float fExpected, string sName, float fWithin = 0.0001)
{
    return Assert(fabs(fActual - fExpected) <= fWithin, sName,
        "got " + FloatToString(fActual, 0, 6) + ", expected " + FloatToString(fExpected, 0, 6));
}

int AssertValid(object oObject, string sName)
{
    return Assert(GetIsObjectValid(oObject), sName, "not a valid object");
}

void TestsDone()
{
    WriteTimestampedLogEntry("MG_TEST DONE");
}
