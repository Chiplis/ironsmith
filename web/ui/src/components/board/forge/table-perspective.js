import * as THREE from 'three';

// Match the DOM card tilt. World Z is height above the tabletop.
export const TABLE_TILT = 12;
export function createTablePerspective() {
  const camera = new THREE.PerspectiveCamera(35, 1, 1, 20000);
  const ray = new THREE.Raycaster();
  let width = 1, height = 1;
  function resize(w, h) {
    width = w; height = h;
    const angle = THREE.MathUtils.degToRad(TABLE_TILT);
    const distance = height / (2 * Math.tan(THREE.MathUtils.degToRad(17.5)) * Math.cos(angle));
    camera.aspect = width / height;
    camera.position.set(width / 2, -height / 2 - distance * Math.sin(angle), -60 + distance * Math.cos(angle));
    camera.lookAt(width / 2, -height / 2, -60);
    camera.updateProjectionMatrix(); camera.updateMatrixWorld(true);
  }
  function point(x, y, z = -60) {
    ray.setFromCamera(new THREE.Vector2(x / width * 2 - 1, 1 - y / height * 2), camera);
    return ray.ray.intersectPlane(new THREE.Plane(new THREE.Vector3(0, 0, 1), -z), new THREE.Vector3());
  }
  function rect(r, z = -60) {
    const corners = [[r.left,r.top],[r.right,r.top],[r.left,r.bottom],[r.right,r.bottom]].map(([x,y])=>point(x,y,z));
    return {left:Math.min(...corners.map(p=>p.x)),right:Math.max(...corners.map(p=>p.x)),top:Math.min(...corners.map(p=>-p.y)),bottom:Math.max(...corners.map(p=>-p.y))};
  }
  resize(1,1);
  return {camera,resize,point,rect};
}
