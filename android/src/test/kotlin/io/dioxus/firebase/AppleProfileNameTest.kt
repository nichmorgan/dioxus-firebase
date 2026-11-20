package io.dioxus.firebase

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AppleProfileNameTest {
    @Test
    fun directName() {
        assertEquals("Ada Lovelace", AppleProfileName.from(mapOf("name" to "Ada Lovelace")))
    }

    @Test
    fun fullNameTrims() {
        assertEquals("Ada Lovelace", AppleProfileName.from(mapOf("fullName" to "  Ada Lovelace  ")))
    }

    @Test
    fun fullNameSnakeCase() {
        assertEquals("Ada Lovelace", AppleProfileName.from(mapOf("full_name" to "Ada Lovelace")))
    }

    @Test
    fun givenAndFamily() {
        assertEquals(
            "Ada Lovelace",
            AppleProfileName.from(mapOf("given_name" to "Ada", "family_name" to "Lovelace")),
        )
    }

    @Test
    fun givenOnly() {
        assertEquals("Ada", AppleProfileName.from(mapOf("given_name" to "Ada")))
    }

    @Test
    fun blankDirectFallsThroughToParts() {
        assertEquals(
            "Ada Lovelace",
            AppleProfileName.from(
                mapOf("name" to "  ", "given_name" to "Ada", "family_name" to "Lovelace"),
            ),
        )
    }

    @Test
    fun blankAndNull() {
        assertNull(AppleProfileName.from(null))
        assertNull(AppleProfileName.from(emptyMap()))
        assertNull(AppleProfileName.from(mapOf("name" to "  ", "given_name" to "", "family_name" to " ")))
    }
}
