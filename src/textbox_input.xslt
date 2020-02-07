<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:svg="http://www.w3.org/2000/svg" version="1.0" xmlns:dyn="http://exslt.org/dynamic">

	<xsl:template match="node()|@*">
		<xsl:copy>
			<xsl:apply-templates select="node()|@*"/>
	    </xsl:copy>
	</xsl:template>


	<xsl:template match="svg:textbox">
		<xsl:variable name="preceding_boxes" select="./preceding::svg:textbox"/>
		<xsl:element name="textbox" namespace="{namespace-uri()}">
			<xsl:attribute name="__id">
            	<xsl:text>textbox-</xsl:text><xsl:value-of select="count($preceding_boxes)"/>
            </xsl:attribute>
            <xsl:copy-of select="./@*" />
            <xsl:apply-templates select="./svg:markup"/>
		</xsl:element>
	</xsl:template>

	<xsl:template match="svg:markup//text()">
		<xsl:value-of select="normalize-space(.)" />
	</xsl:template>

	<xsl:template match="svg:preserved-space">
		<xsl:value-of select="' '" />
	</xsl:template>

	<xsl:strip-space elements="*"/>

	<xsl:template match="svg:br">
		<xsl:text>&#xA;</xsl:text>
	</xsl:template>

	<xsl:template match="svg:divider">
		<xsl:element name="span" namespace="{namespace-uri()}">
			<xsl:attribute name="font-family">
				<xsl:value-of select="'Spectral'"/>
			</xsl:attribute>
		<xsl:text>―――</xsl:text>
		</xsl:element>
	</xsl:template>

</xsl:stylesheet>
