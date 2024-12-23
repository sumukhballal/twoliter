package main

// This module provides simple C FFI bindings over the behavior of the `krane` binary from
// [google/go-containerregistry].
//
// Arguments are provided as a C command line would be: an array of nul-terminated C strings
// alongside an integer representing the length of the array (argc/argv).
//
// The `krane` function also accepts two **C.char pointers which it will point to nul-terminated
// C strings representing the stdout and stderr of the called command.
//
// Note: These strings must be freed by the caller.
//
// [google/go-containerregistry]: https://github.com/google/go-containerregistry

import (
	"io"
	"os"

	"C"
	"unsafe"

	ecr "github.com/awslabs/amazon-ecr-credential-helper/ecr-login"
	"github.com/chrismellard/docker-credential-acr-env/pkg/credhelper"
	"github.com/google/go-containerregistry/cmd/crane/cmd"
	"github.com/google/go-containerregistry/pkg/authn"
	"github.com/google/go-containerregistry/pkg/authn/github"
	"github.com/google/go-containerregistry/pkg/crane"
	"github.com/google/go-containerregistry/pkg/logs"
	"github.com/google/go-containerregistry/pkg/v1/google"
)
import (
	"bytes"
)

func init() {
	logs.Warn.SetOutput(os.Stderr)
	logs.Progress.SetOutput(os.Stderr)
}

//export krane
func krane(argc C.int, argv **C.char, stdout **C.char, stderr **C.char) C.int {
	args := parseCArgs(argc, argv)

	var outBuffer, errBuffer bytes.Buffer

	statusCode := kraneMain(args, false, &outBuffer, &errBuffer)

	*stdout = C.CString(outBuffer.String())
	*stderr = C.CString(errBuffer.String())

	return C.int(statusCode)
}

//export krane_inherited_io
func krane_inherited_io(argc C.int, argv **C.char) C.int {
	args := parseCArgs(argc, argv)

	statusCode := kraneMain(args, true, nil, nil)

	return C.int(statusCode)
}

func parseCArgs(argc C.int, argv **C.char) []string {
	args := make([]string, 0, argc)
	for i := 0; i < int(argc); i++ {
		cStr := C.GoString(*argv)
		args = append(args, cStr)
		argv = (**C.char)(unsafe.Pointer(uintptr(unsafe.Pointer(argv)) + unsafe.Sizeof(*argv)))
	}
	return args
}

const (
	use   = "krane"
	short = "krane is a tool for managing container images"
)

var (
	amazonKeychain authn.Keychain = authn.NewKeychainFromHelper(ecr.NewECRHelper(ecr.WithLogger(io.Discard)))
	azureKeychain  authn.Keychain = authn.NewKeychainFromHelper(credhelper.NewACRCredentialsHelper())
)

func kraneMain(args []string, inherited bool, outBuffer *bytes.Buffer, errBuffer *bytes.Buffer) uint {
	keychain := authn.NewMultiKeychain(
		authn.DefaultKeychain,
		google.Keychain,
		github.Keychain,
		amazonKeychain,
		azureKeychain,
	)

	// Same as crane, but override usage and keychain.
	root := cmd.New(use, short, []crane.Option{crane.WithAuthFromKeychain(keychain)})
	root.SetArgs(args)
	if !inherited {
		root.SetOut(outBuffer)
		root.SetErr(errBuffer)
	}

	if err := root.Execute(); err != nil {
		return 1
	} else {
		return 0
	}
}

func main() {}
