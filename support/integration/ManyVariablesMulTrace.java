public class ManyVariablesMulTrace {
  public static void main(String[] args) {
    int a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t;
    a = b = c = d = e = f = g = h = i = j = k = l = m = n = o = p = q = r = s = t = 0;
    int offset;
    for (int index = 0; index < 5000; index++) {
      if (index % 2 == 0) {
        offset = 1;
      } else if (index % 3 == 0) {
        offset = 2;
      } else if (index % 5 == 0) {
        offset = 3;
      } else {
        offset = 4;
      }
      a += offset;
      b += offset;
      c += offset;
      d += offset;
      e += offset;
      f += offset;
      g += offset;
      h += offset;
      i += offset;
      j += offset;
      k += offset;
      l += offset;
      m += offset;
      n += offset;
      o += offset;
      p += offset;
      q += offset;
      r += offset;
      s += offset;
      t += offset;
    }
    System.out.println(
        a + b + c + d + e + f + g + h + i + j + k + l + m + n + o + p + q + r + s + t);
  }
}