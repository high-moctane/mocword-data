package mocword

import (
	"testing"
)

func TestRegexp(t *testing.T) {
	if reg.MatchString("mar goejc tiej") {
		t.Errorf("returns true")
	}
	if !reg.MatchString("_NOUN_") {
		t.Errorf("returns false _NOUN_")
	}
	if !reg.MatchString("ricgj_ADJ") {
		t.Errorf("returns false ricgj_ADJ")
	}
}
