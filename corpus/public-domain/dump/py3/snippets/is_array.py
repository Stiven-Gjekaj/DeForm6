def is_array(array, index):
    """Returns true or false depending on if an index exists within an array."""
    try:
        array[index]
    except IndexError:
        return False
    return True

# EOF