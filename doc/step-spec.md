# STEP Specification

## Basic Types
* String: A string of characters.
* Float: A floating point number.
* Ref\<T\>: A reference to an entity of type T, e.g. #123.
* Point: A tuple of three floats representing a point in 3D space, e.g. (1.0, 2.0, 3.0).
* Vector3: A tuple of three floats representing a vector in 3D space, e.g. (1.0, 1.0, 0.0).
* NrmVector3: A tuple of three floats representing a normalized vector in 3D space, e.g. (1.0, 0.0, 0.0).

## Functions

### CARTESIAN_POINT(label: String, p: Point)
Defines a cartesian point with the given label and coordinates.
* label: The label of the cartesian point. Is optional and can be ''.
* p: The coordinates of the cartesian point.

### DIRECTION(label: String, v: NrmVector3)
Defines a direction, i.e. a vector, with the given label and coordinates.
* label: The label of the direction. Is optional and can be ''.
* v: The normalized vector defining the direction.

### AXIS2_PLACEMENT_3D(label, pos: Ref\<CARTESIAN_POINT\>, axis0: Ref\<DIRECTION\>, axis1: Ref\<DIRECTION\>)
This entity defines a coordinate system in 3D space using a point for location and two directions for orientation

### APPLICATION_CONTEXT(Description: String)
This entity specifies the context in which the associated product data is intended to be used. It defines the scope or domain (such as automotive, aerospace, electronics, etc.) to ensure that all parties involved in the exchange of data have a common understanding of the intended use and constraints of the data
* Description: A description of the application context.