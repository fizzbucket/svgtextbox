<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:svg="http://www.w3.org/2000/svg" version="1.0">
	
	<xsl:output omit-xml-declaration="yes"/>

	<xsl:template match="svg:use/@xlink:href">
		<xsl:param name="prefix" select="//processing-instruction('svgtextbox-prefix')[1]"/>
		<xsl:param name="joined" select="concat('#', normalize-space($prefix), '-', substring(., 2))"/>
		<xsl:attribute name="xlink:href">
			<xsl:value-of select="$joined"/>
		</xsl:attribute>
	</xsl:template>

	<xsl:template match="svg:symbol/@id">
		<xsl:param name="prefix" select="//processing-instruction('svgtextbox-prefix')[1]"/>
		<xsl:param name="joined" select="concat(normalize-space($prefix), '-', .)"/>
		<xsl:attribute name="id">
			<xsl:value-of select="$joined"/>
		</xsl:attribute>
	</xsl:template>

	<xsl:template match="svg:g/@id">
		<xsl:param name="prefix" select="//processing-instruction('svgtextbox-prefix')[1]"/>
		<xsl:attribute name="id">
			<xsl:choose>
				<xsl:when test="starts-with(., 'surface')">
					<xsl:value-of select="concat(normalize-space($prefix), '-', 'surface')"/>
				</xsl:when>
				<xsl:otherwise>
					<xsl:value-of select="."/>
				</xsl:otherwise>
			</xsl:choose>
		</xsl:attribute>
	</xsl:template>

	<xsl:template match="*/@x">
		<xsl:param name="a" select="//processing-instruction('svgtextbox-x_offset')[1]"/>
		<xsl:param name="b" select="//processing-instruction('svgtextbox-y_offset')[1]"/>
		<xsl:param name="c" select="normalize-space($a)"/>
		<xsl:param name="d" select="normalize-space($b)"/>
		<xsl:param name="e" select="number($c)"/>
		<xsl:param name="f" select="number($d)"/>
		<xsl:param name="offset" select="concat('translate(', $e, ',', $f, ')')"/>
		<xsl:attribute name="x">
			<xsl:value-of select="."/>
		</xsl:attribute>
		<xsl:attribute name="transform">
			<xsl:value-of select="$offset"/>
		</xsl:attribute>
	</xsl:template>


	<xsl:template match="node()|@*">
		<xsl:copy>
			<xsl:apply-templates select="node()|@*"/>
	    </xsl:copy>
	</xsl:template>

</xsl:stylesheet>

