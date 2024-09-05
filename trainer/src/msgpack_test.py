from msgpack import unpack
import sys

# Open the msgpack file
with open(sys.argv[1], 'rb') as file:
    # Decode the msgpack data
    data = unpack(file)

# Print the decoded data
print(data)
