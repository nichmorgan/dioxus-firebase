package io.dioxus.firebase

/**
 * Name from an Apple OAuth `additionalUserInfo.profile` map.
 *
 * Custom Tabs have no `fullName`. Apple sends the name only on the first
 * authorization for that Services ID, so a blank Firebase `displayName` is
 * filled from this map once.
 */
internal object AppleProfileName {
    fun from(profile: Map<String, Any>?): String? {
        if (profile == null) return null
        val direct = listOf("name", "full_name", "fullName")
            .mapNotNull { profile[it] as? String }
            .firstOrNull { it.isNotBlank() }
        if (!direct.isNullOrBlank()) return direct.trim()
        val given = (profile["given_name"] as? String)?.trim().orEmpty()
        val family = (profile["family_name"] as? String)?.trim().orEmpty()
        val joined = "$given $family".trim()
        return joined.ifEmpty { null }
    }
}
