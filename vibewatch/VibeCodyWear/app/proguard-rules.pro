# R8 rules for the Wear OS release build.
#
# MainActivity and the three TileServices are declared in AndroidManifest.xml,
# and AGP generates keep rules for manifest components automatically — they do
# not need entries here.
#
# JSON is parsed with org.json (JSONObject/JSONArray) by hand, not by a
# reflective mapper, so no model classes need keeping. If a reflective
# serializer is ever introduced, its DTOs must be kept explicitly.
#
# OkHttp, okhttp-sse, Play Services Wearable, androidx.security.crypto (Tink)
# and kotlinx-coroutines all ship consumer ProGuard rules inside their AARs,
# which R8 applies automatically.

# Keep the line numbers so release stack traces stay readable, and hide the
# original source file name.
-keepattributes SourceFile,LineNumberTable
-renamesourcefileattribute SourceFile
